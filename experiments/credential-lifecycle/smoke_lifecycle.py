"""Real loopback HTTP/MCP lifecycle, fixed dummy inputs only."""
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

from lifecycle_core import TOKENS

ROOT = Path(__file__).resolve().parent


def payload(raw):
    result = json.loads(raw)["result"]
    return result.get("structuredContent") or json.loads(result["content"][0]["text"])


def main():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    process = subprocess.Popen([sys.executable, str(ROOT / "lifecycle_server.py")],
        env={**os.environ, "OPTIC_LIFECYCLE_PORT": str(port), "OPTIC_LIFECYCLE_TUNNEL": "0"},
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
    sequence = 0
    canary = "REAL-CREDENTIAL-CANARY"
    def request(value, method="tools/call", args=None):
        nonlocal sequence
        sequence += 1
        params = {"name": "credential_probe", "arguments": {}} if method == "tools/call" else {}
        if args is not None:
            params = args
        params["_meta"] = {"io.modelcontextprotocol/protocolVersion": "2026-07-28",
                           "io.modelcontextprotocol/clientCapabilities": {}}
        body = json.dumps({"jsonrpc": "2.0", "id": sequence, "method": method, "params": params})
        connection.request("POST", "/mcp", body, {"Authorization": value,
            "X-Optic-Probe": "optic-probe-dummy-header", "X-Optic-Probe-Secret": "optic-probe-dummy-secret",
            "Content-Type": "application/json", "Accept": "application/json, text/event-stream",
            "MCP-Protocol-Version": "2026-07-28", "MCP-Method": method,
            **({"MCP-Name": params["name"]} if method == "tools/call" else {})})
        response = connection.getresponse()
        raw = response.read()
        assert canary.encode() not in raw
        return response.status, raw
    def token(owner="A", version=0):
        return "Bearer " + TOKENS[owner][version].decode()
    def control(command):
        process.stdin.write(command + "\n")
        process.stdin.flush()
        reply = acknowledgements.get(timeout=5)
        assert reply["ok"]
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
        status, raw = request(token(), "tools/list")
        assert status == 200, f"discovery HTTP {status}"
        assert {t["name"] for t in json.loads(raw)["result"]["tools"]} == {"credential_probe"}
        status, raw = request(token())
        assert status == 200
        first = payload(raw)
        assert first["ok"] and first["authority"] == "none"
        live_socket = connection.sock
        assert request("Bearer " + canary)[0] == 401
        assert request(token(), args={"name": "rotate-a", "arguments": {}})[0] == 400
        assert request(token(), args={"name": "probe_consume_handle", "arguments": {"application_handle": "oph_" + "a" * 64}})[0] == 400
        control("rotate-a")
        assert request(token())[0] == 401
        assert connection.sock is live_socket, "keep-alive socket changed during rotation proof"
        status, raw = request(token(version=1))
        assert status == 200
        rotated = payload(raw)
        assert rotated["principal_id"] == first["principal_id"] and rotated["auth_epoch"] > first["auth_epoch"]
        connection.close()
        connection.connect()
        assert request(token())[0] == 401, "reconnect resurrected old credential"
        assert request(token(version=1))[0] == 200
        control("revoke-a")
        assert request(token(version=1))[0] == 401
        assert request(token())[0] == 401
        status, raw = request(token("B"))
        assert status == 200 and payload(raw)["principal_id"] != first["principal_id"]
        connection.close()
        connection.connect()
        assert request(token(version=1))[0] == 401, "reconnect resurrected revoked credential"
    finally:
        connection.close()
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        reader.join(timeout=5)
    output = "".join(logs)
    assert canary not in output
    assert all(v.decode() not in output for values in TOKENS.values() for v in values)
    assert first["probe_invocation"]["marker"] in output
    assert rotated["probe_invocation"]["marker"] in output
    print("PASS: actual MCP allowlist, local-only rotate/revoke, same keep-alive socket denial,")
    print("reconnect denial, owner independence, principal/epoch separation and credential log absence.")
    print("Synthetic local proof only: no production credential persistence or FranceStudent lifecycle claim.")


if __name__ == "__main__":
    main()
