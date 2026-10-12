from __future__ import annotations

import argparse
import asyncio
import json
import secrets
from typing import Any

from mcp import Client

DEFAULT_URL = "http://127.0.0.1:8000/mcp"


def _content_to_json(result: Any) -> Any:
    structured = getattr(result, "structured_content", None)
    if structured is not None:
        return structured
    structured = getattr(result, "structuredContent", None)
    if structured is not None:
        return structured
    content = getattr(result, "content", None) or []
    for item in content:
        text = getattr(item, "text", None)
        if isinstance(text, str):
            try:
                return json.loads(text)
            except json.JSONDecodeError:
                return text
    return repr(result)


def _require_invocation(payload: Any, tool_name: str) -> str:
    if not isinstance(payload, dict):
        raise RuntimeError(f"{tool_name} returned a non-object result: {payload!r}")
    audit = payload.get("probe_invocation")
    if not isinstance(audit, dict):
        raise RuntimeError(f"{tool_name} result lacks probe_invocation: {payload!r}")
    if audit.get("tool") != tool_name:
        raise RuntimeError(f"{tool_name} invocation proof mismatch: {audit!r}")
    marker = audit.get("marker")
    if not isinstance(marker, str) or len(marker) != 16:
        raise RuntimeError(f"{tool_name} invocation marker is invalid: {audit!r}")
    server_instance = audit.get("server_instance")
    if not isinstance(server_instance, str) or len(server_instance) != 12:
        raise RuntimeError(f"{tool_name} server instance is invalid: {audit!r}")
    return server_instance


async def main() -> int:
    parser = argparse.ArgumentParser(description="Optic H-01 host-probe local self-test")
    parser.add_argument("url", nargs="?", default=DEFAULT_URL)
    args = parser.parse_args()
    run_id = "local-" + secrets.token_hex(8)
    parallel_run_id = "parallel-" + secrets.token_hex(8)

    report: dict[str, Any] = {
        "url": args.url,
        "run_id": run_id,
        "parallel_run_id": parallel_run_id,
    }
    async with Client(args.url) as client:
        report["protocol_version"] = str(client.protocol_version)
        report["server"] = client.server_info.name if client.server_info is not None else None

        tools = await client.list_tools()
        names = sorted(tool.name for tool in tools.tools)
        report["tools"] = names
        required = {
            "probe_ping",
            "probe_host_context",
            "probe_session_correlation",
            "probe_reset",
            "probe_parallel",
        }
        missing = sorted(required.difference(names))
        if missing:
            raise RuntimeError(f"missing probe tools: {missing}")

        ping = _content_to_json(await client.call_tool("probe_ping", {}))
        server_instance = _require_invocation(ping, "probe_ping")
        report["ping"] = ping

        correlation_1 = _content_to_json(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )
        correlation_2 = _content_to_json(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )
        if _require_invocation(correlation_1, "probe_session_correlation") != server_instance:
            raise RuntimeError("first correlation call crossed server instances")
        if _require_invocation(correlation_2, "probe_session_correlation") != server_instance:
            raise RuntimeError("second correlation call crossed server instances")
        if correlation_1.get("call_index") != 1 or correlation_2.get("call_index") != 2:
            raise RuntimeError("correlation store did not produce call_index 1 -> 2")
        report["correlation_1"] = correlation_1
        report["correlation_2"] = correlation_2

        left_raw, right_raw = await asyncio.gather(
            client.call_tool(
                "probe_parallel", {"run_id": parallel_run_id, "delay_ms": 200}
            ),
            client.call_tool(
                "probe_parallel", {"run_id": parallel_run_id, "delay_ms": 200}
            ),
        )
        left = _content_to_json(left_raw)
        right = _content_to_json(right_raw)
        if _require_invocation(left, "probe_parallel") != server_instance:
            raise RuntimeError("left parallel call crossed server instances")
        if _require_invocation(right, "probe_parallel") != server_instance:
            raise RuntimeError("right parallel call crossed server instances")
        if left.get("run_id") != parallel_run_id or right.get("run_id") != parallel_run_id:
            raise RuntimeError(f"parallel run id mismatch: left={left!r}, right={right!r}")
        if left.get("overlap_observed") is not True or right.get("overlap_observed") is not True:
            raise RuntimeError(f"parallel overlap was not observed: left={left!r}, right={right!r}")
        report["parallel"] = [left, right]

        reset = _content_to_json(await client.call_tool("probe_reset", {"run_id": run_id}))
        if _require_invocation(reset, "probe_reset") != server_instance:
            raise RuntimeError("reset crossed server instances")
        if reset.get("removed") is not True:
            raise RuntimeError(f"reset did not remove run: {reset!r}")
        report["reset"] = reset

    print(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
