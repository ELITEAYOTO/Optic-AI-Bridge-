"""Loopback-only real MCP proof; no remote URL or user credential accepted."""
import http.client
import json
import os
from pathlib import Path
import queue
import socket
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / "credential-lifecycle"))
from lifecycle_core import TOKENS
from smoke_lifecycle import payload


def main():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    process = subprocess.Popen([sys.executable, str(ROOT / "bound_server.py")],
        env={**os.environ, "OPTIC_EPOCH_HANDLE_PORT": str(port), "OPTIC_EPOCH_HANDLE_TUNNEL": "0"},
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    logs, acknowledgements = [], queue.Queue(maxsize=16)
    def drain():
        for line in process.stdout:
            if len(logs) < 1024:
                logs.append(line[:4096])
            if line.startswith("[LOCAL] "):
                acknowledgements.put_nowait(json.loads(line.split(" ", 1)[1]))
    reader = threading.Thread(target=drain, daemon=True)
    reader.start()
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    seq, minted, proofs = 0, [], []
    canary = "REAL-CREDENTIAL-CANARY"
    def request(name, arguments=None, owner="A", version=0, method="tools/call", credential=None):
        nonlocal seq
        seq += 1
        params = {"name": name, "arguments": arguments or {}} if method == "tools/call" else {}
        params["_meta"] = {"io.modelcontextprotocol/protocolVersion": "2026-07-28",
                           "io.modelcontextprotocol/clientCapabilities": {}}
        connection.request("POST", "/mcp", json.dumps({"jsonrpc": "2.0", "id": seq, "method": method, "params": params}),
            {"Authorization": credential or "Bearer " + TOKENS[owner][version].decode(),
             "X-Optic-Probe": "optic-probe-dummy-header", "X-Optic-Probe-Secret": "optic-probe-dummy-secret",
             "Content-Type": "application/json", "Accept": "application/json, text/event-stream",
             "MCP-Protocol-Version": "2026-07-28", "MCP-Method": method,
             **({"MCP-Name": name} if method == "tools/call" else {})})
        response = connection.getresponse()
        raw = response.read()
        assert canary.encode() not in raw
        return response.status, raw
    def tool(name, args=None, owner="A", version=0):
        status, raw = request(name, args, owner, version)
        assert status == 200
        value = payload(raw)
        proofs.append(value["probe_invocation"])
        return value
    def create(owner="A", version=0):
        value = tool("bound_create_handle", owner=owner, version=version)
        assert value["ok"] and value["authority"] == "none"
        minted.append(value["application_handle"])
        return value
    def consume(handle, owner="A", version=0):
        return tool("bound_consume_handle", {"application_handle": handle}, owner, version)
    def control(command):
        process.stdin.write(command + "\n")
        process.stdin.flush()
        assert acknowledgements.get(timeout=5)["ok"]
    try:
        deadline = time.monotonic() + 20
        while True:
            assert process.poll() is None, "server startup failed"
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                assert time.monotonic() < deadline
                time.sleep(0.1)
        status, raw = request("", method="tools/list")
        assert status == 200
        assert {t["name"] for t in json.loads(raw)["result"]["tools"]} == {"bound_create_handle", "bound_consume_handle"}
        assert request("rotate-a")[0] == 400
        assert request("credential_probe")[0] == 400
        assert request("bound_create_handle", credential="Bearer " + canary)[0] == 401
        first = create()
        handle = first["application_handle"]
        assert not consume(handle, owner="B")["ok"]
        connection.close()
        connection.connect()
        assert consume(handle)["passed_unchanged"]
        assert not consume(handle)["ok"]
        old = create()
        live_socket = connection.sock
        control("rotate-a")
        assert request("bound_consume_handle", {"application_handle": old["application_handle"]})[0] == 401
        assert connection.sock is live_socket
        # Same principal but NEW epoch cannot recover an OLD binding.
        denied = consume(old["application_handle"], version=1)
        assert denied["code"] == "probe.invalid_handle"
        fresh = create(version=1)
        assert fresh["principal_id"] == first["principal_id"] and fresh["auth_epoch"] > first["auth_epoch"]
        assert consume(fresh["application_handle"], version=1)["ok"]
        pending_a = create(version=1)
        pending_b = create(owner="B")
        control("revoke-a")
        assert request("bound_consume_handle", {"application_handle": pending_a["application_handle"]}, version=1)[0] == 401
        assert not consume(pending_a["application_handle"], owner="B")["ok"]
        assert consume(pending_b["application_handle"], owner="B")["ok"]
        connection.close()
        connection.connect()
        assert request("bound_consume_handle", {"application_handle": pending_a["application_handle"]}, version=1)[0] == 401
    finally:
        connection.close()
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        reader.join(timeout=5)
    text = "".join(logs)
    assert canary not in text
    assert all(v.decode() not in text for values in TOKENS.values() for v in values)
    assert all(value not in text for value in minted)
    assert all(json.dumps(proof, sort_keys=True) in text for proof in proofs)
    print("PASS: real two-tool MCP, exact handoff/replay, other-owner denial without token consumption,")
    print("old epoch invalidation with new valid credential, local rotate/revoke, reconnect and B independence.")
    print("No raw credential/handle in logs. Local synthetic integration only, not production SessionResolver.")


if __name__ == "__main__":
    main()
