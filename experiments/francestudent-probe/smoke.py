"""Local-only real MCP smoke. Never accepts a remote URL or a real credential."""
import asyncio
from contextlib import asynccontextmanager
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen

import httpx2
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client
from probe_core import DUMMY_HEADERS, DUMMY_TOKENS

ROOT = Path(__file__).resolve().parent


def header_map(owner="A"):
    return {"Authorization": "Bearer " + DUMMY_TOKENS[owner].decode(),
            **{k.decode(): v.decode() for k, v in DUMMY_HEADERS.items()}}


@asynccontextmanager
async def client(url, owner="A"):
    async with httpx2.AsyncClient(headers=header_map(owner), timeout=10) as http:
        async with streamable_http_client(url, http_client=http) as (read, write):
            async with ClientSession(read, write, read_timeout_seconds=10) as session:
                await session.initialize()
                yield session


def structured(response):
    assert not response.is_error
    if response.structured_content is not None:
        return response.structured_content
    return json.loads(next(item.text for item in response.content if item.type == "text"))


async def exercise(url):
    proofs = []
    async with client(url) as session:
        tools = await session.list_tools()
        assert {t.name for t in tools.tools} == {"probe_headers", "probe_create_handle", "probe_consume_handle"}
        headers = structured(await session.call_tool("probe_headers", {}))
        assert headers["request_observation"]["matches_dummy"]
        proofs.append(headers["probe_invocation"])
        created = structured(await session.call_tool("probe_create_handle", {}))
        assert created["ok"]
        handle = created["application_handle"]
        proofs.append(created["probe_invocation"])
    # Fresh transport, same server and owner. Connection continuity is not identity.
    async with client(url, "B") as other:
        denied = structured(await other.call_tool("probe_consume_handle", {"application_handle": handle}))
        assert denied["code"] == "probe.invalid_handle"
        proofs.append(denied["probe_invocation"])
    async with client(url) as reconnected:
        consumed = structured(await reconnected.call_tool("probe_consume_handle", {"application_handle": handle}))
        assert consumed["ok"] and consumed["passed_unchanged"]
        proofs.append(consumed["probe_invocation"])
        replay = structured(await reconnected.call_tool("probe_consume_handle", {"application_handle": handle}))
        assert replay["code"] == "probe.invalid_handle"
        proofs.append(replay["probe_invocation"])
        # Two concurrent attempts on one handle: exactly one can consume it.
        created = structured(await reconnected.call_tool("probe_create_handle", {}))
        results = await asyncio.gather(*[reconnected.call_tool("probe_consume_handle",
            {"application_handle": created["application_handle"]}) for _ in range(2)])
        assert sum(structured(r)["ok"] for r in results) == 1
    return proofs, handle


def main():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = {**os.environ, "OPTIC_FS_PROBE_PORT": str(port), "OPTIC_FS_PROBE_RANDOM_TUNNEL": "0"}
    process = subprocess.Popen([sys.executable, str(ROOT / "server.py")], env=env,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    logs = []
    def drain():
        for line in process.stdout:
            if len(logs) < 1024:
                logs.append(line[:4096])
    reader = threading.Thread(target=drain, daemon=True)
    reader.start()
    canary = "REAL-BEARER-CANARY-never-log"
    handle = ""
    try:
        deadline = time.monotonic() + 20
        while True:
            assert process.poll() is None, "probe exited before listening"
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                assert time.monotonic() < deadline, "probe startup deadline"
                time.sleep(0.1)
        url = f"http://127.0.0.1:{port}/mcp"
        cases = [({}, b"{}", 401), ({"Authorization": "Bearer " + canary}, b"{}", 401),
                 ({**header_map(), "X-Optic-Probe-Secret": canary}, b"{}", 401),
                 (header_map(), b"x" * (65536 + 1), 413),
                 (header_map(), b'{"method":"tools/call","params":{"name":"session_cancel"}}', 400)]
        for h, body, status in cases:
            request = Request(url, body, {**h, "Content-Type": "application/json"}, method="POST")
            try:
                urlopen(request, timeout=5)
                raise AssertionError("denied request accepted")
            except HTTPError as error:
                assert error.code == status
                assert canary.encode() not in error.read()
        # Distinguish missing headers at ingress from unsupported SDK methods,
        # without guessing that these caused the historical FranceStudent rows.
        for method, h, status in (("GET", {"Authorization": header_map()["Authorization"]}, 401),
                                  ("DELETE", header_map(), 405)):
            request = Request(url, headers={**h, "MCP-Protocol-Version": "2026-07-28"}, method=method)
            try:
                urlopen(request, timeout=5)
                raise AssertionError("method probe unexpectedly accepted")
            except HTTPError as error:
                assert error.code == status
        proofs, handle = asyncio.run(exercise(url))
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        reader.join(timeout=5)
    text = "".join(logs)
    assert canary not in text
    assert handle not in text
    assert all(v.decode() not in text for v in DUMMY_TOKENS.values())
    for proof in proofs:
        assert json.dumps(proof, sort_keys=True) in text, "tool result lacks matching server proof"
    http_proofs = [json.loads(line.split(" HTTP ", 1)[1]) for line in logs if " HTTP " in line]
    assert len(http_proofs) > len(proofs), "transport discovery requests missing from proof"
    assert len({p["sequence"] for p in http_proofs}) == len(http_proofs)
    by_sequence = {p["sequence"]: p for p in http_proofs}
    statuses = [json.loads(line.split(" STATUS ", 1)[1]) for line in logs if " STATUS " in line]
    assert all(p["proof_version"] == "fs-probe-v2" for p in http_proofs)
    assert any(by_sequence[s["sequence"]]["http_method"] == "GET" and s["stage"] == "dummy_custom_headers"
               and s["status"] == 401 for s in statuses)
    assert any(by_sequence[s["sequence"]]["http_method"] == "DELETE" and s["stage"] == "mcp"
               and s["status"] == 405 for s in statuses)
    dispatch = [json.loads(line.split(" DISPATCH ", 1)[1]) for line in logs if " DISPATCH " in line]
    assert {"initialize", "tools/list", "tools/call"}.issubset({p["method"] for p in dispatch})
    for item in dispatch:
        observation = by_sequence[item["sequence"]]["observation"]
        assert observation["matches_dummy"] and observation["scheme"] == "Bearer"
        assert all(v["matches_dummy"] for v in observation["custom"].values())
    print("PASS: dummy headers on discovery/tool/reconnect requests; owner denial; exact handoff;")
    print("single use; atomic race; bounded ingress; raw credential/handle absence from server logs.")
    print("PASS: safe HTTP-method and rejection-stage diagnostics (fs-probe-v2).")
    print("Local SDK proof only; this run adds no FranceStudent live measurement.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
