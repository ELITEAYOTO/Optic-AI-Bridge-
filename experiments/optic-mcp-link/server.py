from __future__ import annotations

import asyncio
import json
import os
import secrets
import tempfile
import threading
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any, AsyncIterator

import uvicorn
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
from mcp.server.mcpserver import Context
from starlette.applications import Starlette
from starlette.routing import Mount

HOST = os.environ.get("OPTIC_REAL_LINK_HOST", "127.0.0.1")
PORT = int(os.environ.get("OPTIC_REAL_LINK_PORT", "8000"))
PATH = os.environ.get("OPTIC_REAL_LINK_PATH", "/mcp")
OPTIC_APP = os.environ.get("OPTIC_REAL_LINK_OPTIC_APP")
ALLOW_RANDOM_TUNNEL_HOST = (
    os.environ.get("OPTIC_REAL_LINK_ALLOW_RANDOM_TUNNEL_HOST", "0") == "1"
)
MAX_REQUEST_BODY_BYTES = 64 * 1024
MAX_CONCURRENT_FORWARDS = 8
MAX_HTTP_CONCURRENCY = 32
FORWARD_ADMISSION_TIMEOUT_SECONDS = 0.25
UPSTREAM_CALL_TIMEOUT_SECONDS = 15.0

mcp = MCPServer(
    "Optic-Real-MCP-Link-Experiment",
    instructions=(
        "Experiment-only adapter to a real optic-bridge stdio server running against a "
        "disposable synthetic workspace. No real user project is exposed."
    ),
)

_upstream: ClientSession | None = None
_upstream_tools: tuple[str, ...] = ()
_forward_gate: asyncio.Semaphore | None = None
_fixture_marker = ""
_adapter_instance = secrets.token_hex(6)
_invocation_lock = threading.Lock()
_invocation_sequence = 0


def _new_invocation(tool_name: str) -> dict[str, Any]:
    global _invocation_sequence
    marker = secrets.token_hex(8)
    with _invocation_lock:
        _invocation_sequence += 1
        sequence = _invocation_sequence
    proof = {
        "tool": tool_name,
        "sequence": sequence,
        "marker": marker,
        "adapter_instance": _adapter_instance,
    }
    print(
        f"[REAL-LINK] TOOL_CALL instance={_adapter_instance} tool={tool_name} "
        f"seq={sequence} marker={marker}",
        flush=True,
    )
    return proof


def _require_upstream() -> ClientSession:
    if _upstream is None:
        raise RuntimeError("optic upstream session is not initialized")
    return _upstream


def _require_forward_gate() -> asyncio.Semaphore:
    if _forward_gate is None:
        raise RuntimeError("optic adapter admission gate is not initialized")
    return _forward_gate


def _structured(result: Any) -> dict[str, Any]:
    for attr in ("structured_content", "structuredContent"):
        value = getattr(result, attr, None)
        if isinstance(value, dict):
            return value

    content = getattr(result, "content", None) or []
    for item in content:
        text = getattr(item, "text", None)
        if isinstance(text, str):
            try:
                decoded = json.loads(text)
            except json.JSONDecodeError:
                continue
            if isinstance(decoded, dict):
                return decoded
    raise RuntimeError("upstream Optic tool did not return structured JSON")


async def _forward(tool_name: str, arguments: dict[str, Any]) -> dict[str, Any]:
    session = _require_upstream()
    gate = _require_forward_gate()
    try:
        await asyncio.wait_for(
            gate.acquire(), timeout=FORWARD_ADMISSION_TIMEOUT_SECONDS
        )
    except TimeoutError as exc:
        raise RuntimeError("optic synthetic adapter is busy") from exc

    try:
        result = await asyncio.wait_for(
            session.call_tool(tool_name, arguments=arguments),
            timeout=UPSTREAM_CALL_TIMEOUT_SECONDS,
        )
    finally:
        gate.release()
    return _structured(result)


def _wrap(tool_name: str, upstream: dict[str, Any]) -> dict[str, Any]:
    return {
        "adapter_invocation": _new_invocation(tool_name),
        "upstream": upstream,
    }


@mcp.tool()
async def optic_session_info(ctx: Context) -> dict[str, Any]:
    """Forward to the real Optic `session_info` tool for the synthetic fixture session."""
    del ctx
    return _wrap("optic_session_info", await _forward("session_info", {}))


@mcp.tool()
async def optic_fs_list(
    ctx: Context,
    path: str | None = None,
    cursor: int | None = None,
    limit: int | None = None,
) -> dict[str, Any]:
    """Forward a bounded directory listing to real Optic inside the synthetic fixture only."""
    del ctx
    arguments: dict[str, Any] = {}
    if path is not None:
        arguments["path"] = path
    if cursor is not None:
        arguments["cursor"] = cursor
    if limit is not None:
        arguments["limit"] = limit
    return _wrap("optic_fs_list", await _forward("fs_list", arguments))


@mcp.tool()
async def optic_fs_read(
    path: str,
    ctx: Context,
    offset: int | None = None,
    max_bytes: int | None = None,
) -> dict[str, Any]:
    """Forward a bounded file read to real Optic inside the synthetic fixture only."""
    del ctx
    arguments: dict[str, Any] = {"path": path}
    if offset is not None:
        arguments["offset"] = offset
    if max_bytes is not None:
        arguments["max_bytes"] = max_bytes
    return _wrap("optic_fs_read", await _forward("fs_read", arguments))


def _write_fixture(root: Path) -> str:
    marker = secrets.token_hex(12)
    (root / "public").mkdir(parents=True)
    (root / "nested").mkdir(parents=True)
    (root / "public" / "hello.txt").write_text(
        "OPTIC_REAL_LINK_SYNTHETIC=1\n" f"fixture_marker={marker}\n",
        encoding="utf-8",
    )
    (root / "nested" / "info.json").write_text(
        json.dumps(
            {
                "kind": "synthetic-fixture",
                "purpose": "real-optic-mcp-link",
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    return marker


def _transport_security() -> tuple[TransportSecuritySettings, str]:
    if ALLOW_RANDOM_TUNNEL_HOST:
        return (
            TransportSecuritySettings(enable_dns_rebinding_protection=False),
            "random tunnel host allowed (synthetic fixture experiment only)",
        )
    return (
        TransportSecuritySettings(
            enable_dns_rebinding_protection=True,
            allowed_hosts=["127.0.0.1:*", "localhost:*", "[::1]:*"],
            allowed_origins=[
                "http://127.0.0.1:*",
                "http://localhost:*",
                "http://[::1]:*",
            ],
        ),
        "DNS-rebinding protection enabled; loopback hosts only",
    )


def _validate_optic_app() -> Path:
    if not OPTIC_APP:
        raise RuntimeError("OPTIC_REAL_LINK_OPTIC_APP must point to the optic-bridge binary")
    path = Path(OPTIC_APP).expanduser().resolve()
    if not path.is_file():
        raise RuntimeError(f"optic-bridge binary not found: {path}")
    return path


security, security_mode = _transport_security()
streamable_app = mcp.streamable_http_app(
    streamable_http_path=PATH,
    stateless_http=True,
    json_response=True,
    max_request_body_size=MAX_REQUEST_BODY_BYTES,
    transport_security=security,
    host=HOST,
)


@asynccontextmanager
async def lifespan(_: Starlette) -> AsyncIterator[None]:
    global _upstream, _upstream_tools, _forward_gate, _fixture_marker

    optic_app = _validate_optic_app()
    with tempfile.TemporaryDirectory(prefix="optic-real-mcp-link-") as temp_root:
        root = Path(temp_root)
        _fixture_marker = _write_fixture(root)
        params = StdioServerParameters(command=str(optic_app), args=[str(root)])

        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as session:
                init = await session.initialize()
                tools = await session.list_tools()
                names = tuple(sorted(tool.name for tool in tools.tools))
                required = {"session_info", "fs_list", "fs_read"}
                missing = required.difference(names)
                if missing:
                    raise RuntimeError(
                        f"real Optic upstream is missing required tools: {sorted(missing)}"
                    )

                _upstream = session
                _upstream_tools = names
                _forward_gate = asyncio.Semaphore(MAX_CONCURRENT_FORWARDS)
                print("=== Optic real MCP link experiment ===", flush=True)
                print(f"Local endpoint : http://{HOST}:{PORT}{PATH}", flush=True)
                print("Remote surface : session_info, fs_list, fs_read only", flush=True)
                print("Workspace      : disposable synthetic fixture only", flush=True)
                print(f"Transport guard: {security_mode}", flush=True)
                print(f"Request limit  : {MAX_REQUEST_BODY_BYTES} bytes", flush=True)
                print(f"Forward limit  : {MAX_CONCURRENT_FORWARDS} calls", flush=True)
                print(f"HTTP limit     : {MAX_HTTP_CONCURRENCY} concurrent tasks", flush=True)
                print(f"Adapter instance: {_adapter_instance}", flush=True)
                print(f"Fixture marker : {_fixture_marker}", flush=True)
                print(f"Upstream server: {init.server_info.name}", flush=True)
                print(f"Upstream tools : {', '.join(names)}", flush=True)
                print(
                    "DO NOT point this experiment at a real project workspace.",
                    flush=True,
                )
                print(flush=True)

                async with mcp.session_manager.run():
                    try:
                        yield
                    finally:
                        _upstream = None
                        _upstream_tools = ()
                        _forward_gate = None
                        _fixture_marker = ""


app = Starlette(routes=[Mount("/", app=streamable_app)], lifespan=lifespan)


if __name__ == "__main__":
    uvicorn.run(
        app,
        host=HOST,
        port=PORT,
        log_level="info",
        limit_concurrency=MAX_HTTP_CONCURRENCY,
        timeout_keep_alive=5,
    )
