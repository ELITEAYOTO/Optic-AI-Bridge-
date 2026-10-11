from __future__ import annotations

import argparse
import asyncio
import base64
from typing import Any

from mcp import Client


def _structured(result: Any) -> dict[str, Any]:
    for attr in ("structured_content", "structuredContent"):
        value = getattr(result, attr, None)
        if isinstance(value, dict):
            return value
    raise AssertionError(f"tool did not return structured content: {result!r}")


def _upstream(payload: dict[str, Any], expected_tool: str) -> dict[str, Any]:
    proof = payload.get("adapter_invocation")
    if not isinstance(proof, dict) or proof.get("tool") != expected_tool:
        raise AssertionError(f"invalid adapter proof: {payload!r}")
    marker = proof.get("marker")
    instance = proof.get("adapter_instance")
    sequence = proof.get("sequence")
    if not isinstance(marker, str) or len(marker) != 16:
        raise AssertionError(f"invalid adapter marker: {proof!r}")
    if not isinstance(instance, str) or len(instance) != 12:
        raise AssertionError(f"invalid adapter instance: {proof!r}")
    if not isinstance(sequence, int) or sequence <= 0:
        raise AssertionError(f"invalid adapter sequence: {proof!r}")
    upstream = payload.get("upstream")
    if not isinstance(upstream, dict):
        raise AssertionError(f"missing upstream result: {payload!r}")
    return upstream


async def _main(url: str) -> None:
    async with Client(url) as client:
        tools = await client.list_tools()
        names = {tool.name for tool in tools.tools}
        expected = {"optic_session_info", "optic_fs_list", "optic_fs_read"}
        if names != expected:
            raise AssertionError(f"unexpected remote tool surface: {sorted(names)}")

        session = _upstream(
            _structured(await client.call_tool("optic_session_info", {})),
            "optic_session_info",
        )
        if session.get("principal") != "local-stdio" or session.get("project") != "local-workspace":
            raise AssertionError(f"unexpected real Optic session: {session!r}")
        capabilities = set(session.get("capabilities") or [])
        if not {"file_read", "file_search"}.issubset(capabilities):
            raise AssertionError(f"missing expected read authority: {session!r}")
        forbidden = {"file_write", "file_delete", "git_read", "git_integrate", "process_run"}
        if capabilities.intersection(forbidden):
            raise AssertionError(f"unexpected synthetic-session authority: {session!r}")

        root = _upstream(
            _structured(await client.call_tool("optic_fs_list", {"limit": 8})),
            "optic_fs_list",
        )
        root_names = {entry.get("name") for entry in root.get("entries") or []}
        if root_names != {"nested", "public"}:
            raise AssertionError(f"unexpected synthetic root: {root!r}")

        public = _upstream(
            _structured(
                await client.call_tool("optic_fs_list", {"path": "public", "limit": 8})
            ),
            "optic_fs_list",
        )
        entries = public.get("entries") or []
        if len(entries) != 1 or entries[0].get("path") != "public/hello.txt":
            raise AssertionError(f"unexpected public fixture listing: {public!r}")

        read = _upstream(
            _structured(
                await client.call_tool(
                    "optic_fs_read", {"path": "public/hello.txt", "max_bytes": 4096}
                )
            ),
            "optic_fs_read",
        )
        decoded = base64.b64decode(read.get("data", ""), validate=True).decode("utf-8")
        if "OPTIC_REAL_LINK_SYNTHETIC=1" not in decoded or "fixture_marker=" not in decoded:
            raise AssertionError(f"unexpected synthetic file content: {decoded!r}")

        left, right = await asyncio.gather(
            client.call_tool(
                "optic_fs_read", {"path": "public/hello.txt", "max_bytes": 4096}
            ),
            client.call_tool(
                "optic_fs_read", {"path": "nested/info.json", "max_bytes": 4096}
            ),
        )
        _upstream(_structured(left), "optic_fs_read")
        _upstream(_structured(right), "optic_fs_read")

        rejected = False
        try:
            await client.call_tool("optic_fs_read", {"path": "../outside.txt"})
        except Exception:
            rejected = True
        if not rejected:
            raise AssertionError("real Optic unexpectedly accepted a traversal path")

    print("real Optic MCP link client smoke passed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("url")
    args = parser.parse_args()
    asyncio.run(_main(args.url))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
