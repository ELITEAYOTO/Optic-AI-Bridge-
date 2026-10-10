from __future__ import annotations

import asyncio
import os
from pathlib import Path
import socket
import subprocess
import sys
import time
from typing import Any

from mcp import Client

ROOT = Path(__file__).resolve().parent


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def _wait_for_port(port: int, process: subprocess.Popen[str], timeout: float = 20.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            output = process.stdout.read() if process.stdout is not None else ""
            raise RuntimeError(f"probe server exited early with {process.returncode}:\n{output}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.25):
                return
        except OSError:
            time.sleep(0.1)
    raise TimeoutError(f"probe server did not listen on port {port} within {timeout}s")


def _structured(result: Any) -> Any:
    for attr in ("structured_content", "structuredContent"):
        value = getattr(result, attr, None)
        if value is not None:
            return value
    return None


async def _exercise(url: str) -> None:
    async with Client(url) as client:
        tools = await client.list_tools()
        names = {tool.name for tool in tools.tools}
        expected = {
            "probe_ping",
            "probe_host_context",
            "probe_session_correlation",
            "probe_reset",
            "probe_parallel",
        }
        missing = expected.difference(names)
        if missing:
            raise AssertionError(f"missing tools: {sorted(missing)}")

        ping = _structured(await client.call_tool("probe_ping", {}))
        if not isinstance(ping, dict) or ping.get("ok") is not True:
            raise AssertionError(f"unexpected ping result: {ping!r}")

        run_id = "ci-host-probe-0001"
        first = _structured(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )
        second = _structured(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )
        if not isinstance(first, dict) or first.get("call_index") != 1:
            raise AssertionError(f"unexpected first correlation result: {first!r}")
        if not isinstance(second, dict) or second.get("call_index") != 2:
            raise AssertionError(f"unexpected second correlation result: {second!r}")

        left, right = await asyncio.gather(
            client.call_tool("probe_parallel", {"delay_ms": 100}),
            client.call_tool("probe_parallel", {"delay_ms": 100}),
        )
        if _structured(left) is None or _structured(right) is None:
            raise AssertionError("parallel probe calls did not return structured data")

        reset = _structured(await client.call_tool("probe_reset", {"run_id": run_id}))
        if not isinstance(reset, dict) or reset.get("removed") is not True:
            raise AssertionError(f"unexpected reset result: {reset!r}")


def main() -> int:
    port = _free_port()
    env = os.environ.copy()
    env["OPTIC_HOST_PROBE_PORT"] = str(port)
    env["OPTIC_HOST_PROBE_ALLOW_RANDOM_TUNNEL_HOST"] = "0"
    process = subprocess.Popen(
        [sys.executable, str(ROOT / "server.py")],
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        _wait_for_port(port, process)
        asyncio.run(_exercise(f"http://127.0.0.1:{port}/mcp"))
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        if process.stdout is not None:
            output = process.stdout.read().strip()
            if output:
                print(output)
    print("H-01 host probe integration smoke passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
