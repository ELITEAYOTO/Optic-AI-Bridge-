"""Actual HTTP/MCP with process-kill/restart of a disposable local fixture."""
import http.client
import json
import os
from pathlib import Path
import queue
import socket
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / "credential-lifecycle"))
from lifecycle_core import TOKENS
from smoke_lifecycle import payload


def main():
    logs, processes, readers = [], [], []
    sequence = 0
    canary = "REAL-CREDENTIAL-CANARY"
    def start(fixture):
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        process = subprocess.Popen([sys.executable, str(ROOT / "persistent_server.py"), "--fixture-dir", str(fixture)],
            env={**os.environ, "OPTIC_PERSISTENT_AUTH_PORT": str(port), "OPTIC_PERSISTENT_AUTH_TUNNEL": "0"},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        processes.append(process)
        acks = queue.Queue(maxsize=16)
        def drain():
            for line in process.stdout:
                if len(logs) < 2048:
                    logs.append(line[:4096])
                if line.startswith("[LOCAL] {"):
                    acks.put_nowait(json.loads(line.split(" ", 1)[1]))
        reader = threading.Thread(target=drain, daemon=True)
        readers.append(reader)
        reader.start()
        deadline = time.monotonic() + 20
        while True:
            assert process.poll() is None, "fixture startup blocked"
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                assert time.monotonic() < deadline
                time.sleep(0.1)
        return process, acks, http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    def request(connection, owner="A", version=0, method="tools/call", credential=None):
        nonlocal sequence
        sequence += 1
        name = "persistent_credential_probe"
        params = {"name": name, "arguments": {}} if method == "tools/call" else {}
        params["_meta"] = {"io.modelcontextprotocol/protocolVersion": "2026-07-28",
                           "io.modelcontextprotocol/clientCapabilities": {}}
        connection.request("POST", "/mcp", json.dumps({"jsonrpc": "2.0", "id": sequence, "method": method, "params": params}),
            {"Authorization": credential or "Bearer " + TOKENS[owner][version].decode(),
             "X-Optic-Probe": "optic-probe-dummy-header", "X-Optic-Probe-Secret": "optic-probe-dummy-secret",
             "Content-Type": "application/json", "Accept": "application/json, text/event-stream",
             "MCP-Protocol-Version": "2026-07-28", "MCP-Method": method,
             **({"MCP-Name": name} if method == "tools/call" else {})})
        response = connection.getresponse()
        raw = response.read()
        assert canary.encode() not in raw
        return response.status, raw
    def control(process, acks, command):
        process.stdin.write(command + "\n")
        process.stdin.flush()
        result = acks.get(timeout=5)
        assert result["ok"] and result["persisted"]
    def kill(process, connection):
        connection.close()
        process.kill()  # intentionally bypass graceful shutdown/finally
        process.wait(timeout=5)
    try:
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory) / "fixture"
            init = subprocess.run([sys.executable, str(ROOT / "persistent_server.py"), "--fixture-dir", str(fixture), "--initialize"],
                capture_output=True, text=True, timeout=20)
            assert init.returncode == 0
            process, acks, connection = start(fixture)
            status, raw = request(connection, method="tools/list")
            assert status == 200 and {t["name"] for t in json.loads(raw)["result"]["tools"]} == {"persistent_credential_probe"}
            status, raw = request(connection)
            assert status == 200
            original = payload(raw)
            assert original["ok"] and original["auth_epoch"] == 1
            assert request(connection, credential="Bearer " + canary)[0] == 401
            control(process, acks, "rotate-a")
            kill(process, connection)
            process, acks, connection = start(fixture)
            assert request(connection)[0] == 401
            status, raw = request(connection, version=1)
            assert status == 200
            rotated = payload(raw)
            assert rotated["principal_id"] == original["principal_id"] and rotated["auth_epoch"] == 2
            control(process, acks, "revoke-a")
            kill(process, connection)
            process, acks, connection = start(fixture)
            assert request(connection)[0] == 401
            assert request(connection, version=1)[0] == 401
            status, raw = request(connection, owner="B")
            assert status == 200 and payload(raw)["auth_epoch"] == 1
            kill(process, connection)
            text = (fixture / "state.json").read_text()
            assert all(token.decode() not in text for values in TOKENS.values() for token in values)
    finally:
        for process in processes:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
        for reader in readers:
            reader.join(timeout=5)
    output = "".join(logs)
    assert canary not in output
    assert all(token.decode() not in output for values in TOKENS.values() for token in values)
    assert original["probe_invocation"]["marker"] in output and rotated["probe_invocation"]["marker"] in output
    print("PASS: acknowledged dummy rotate/revoke survive actual process kill/restart,")
    print("same independent principal/advanced epoch, revoked old/new denied and owner B unaffected.")
    print("No raw credentials in persisted fixture or logs. Not production/power-loss durability proof.")


if __name__ == "__main__":
    main()
