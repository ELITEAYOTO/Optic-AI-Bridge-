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


async def main() -> int:
    parser = argparse.ArgumentParser(description="Optic H-01 host-probe local self-test")
    parser.add_argument("url", nargs="?", default=DEFAULT_URL)
    args = parser.parse_args()
    run_id = "local-" + secrets.token_hex(8)

    report: dict[str, Any] = {"url": args.url, "run_id": run_id}
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

        report["ping"] = _content_to_json(await client.call_tool("probe_ping", {}))
        report["correlation_1"] = _content_to_json(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )
        report["correlation_2"] = _content_to_json(
            await client.call_tool("probe_session_correlation", {"run_id": run_id})
        )

        left, right = await asyncio.gather(
            client.call_tool("probe_parallel", {"delay_ms": 200}),
            client.call_tool("probe_parallel", {"delay_ms": 200}),
        )
        report["parallel"] = [_content_to_json(left), _content_to_json(right)]
        report["reset"] = _content_to_json(
            await client.call_tool("probe_reset", {"run_id": run_id})
        )

    print(json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
