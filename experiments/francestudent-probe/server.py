from __future__ import annotations

import asyncio
from contextvars import ContextVar
import json
import logging
import os
import secrets
import time
from datetime import datetime, timezone

from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn

from probe_core import HANDLE_RE, Handles, observe_headers

MAX_BODY = 64 * 1024
MAX_HEADERS = 16 * 1024
MAX_ACTIVE = 8
MAX_REQUESTS = 512
MAX_LIFETIME = 25 * 60
PROOF_VERSION = "fs-probe-v2"
request_proof = ContextVar("request_proof", default=None)


def timestamp_utc():
    # Display/correlation only; authorization and durations remain monotonic.
    return datetime.now(timezone.utc).isoformat(timespec='milliseconds')
handles = Handles()
mcp = MCPServer("Optic-FranceStudent-Dummy-Probe", instructions=(
    "Authority-free synthetic probe. Call probe_create_handle then pass the returned "
    "application_handle unchanged to probe_consume_handle within 120 seconds. "
    "Never supply credentials as tool arguments. No files, processes, Git or runtime."))


def result(tool, payload):
    proof = request_proof.get()
    if proof is None:
        return {"ok": False, "code": "probe.transport_required"}
    invocation = {"tool": tool, "marker": secrets.token_hex(8),
                  "server_instance": proof["server_instance"], "request_sequence": proof["sequence"]}
    if proof.get('timed_diagnostics'):
        invocation['timestamp_utc'] = timestamp_utc()
        now = proof['clock']()
        invocation['elapsed_ms'] = round(max(0, now - proof['started']) * 1000, 3)
        invocation['request_elapsed_ms'] = round(max(0, now - proof['admitted']) * 1000, 3)
    line = "[FS-PROBE] TOOL " + json.dumps(invocation, sort_keys=True)
    emitter = proof.get('tool_emit')
    if emitter is None:
        print(line, flush=True)
    else:
        queued = emitter(line)
        if proof.get('timed_diagnostics'):
            invocation['console_queued'] = queued is not False
    return {**payload, "probe_invocation": invocation, "request_observation": proof["observation"]}


@mcp.tool()
def probe_headers() -> dict:
    """Report only presence, Bearer scheme and exact dummy-match bits for this request."""
    return result("probe_headers", {"ok": True, "authority": "none"})


@mcp.tool()
def probe_create_handle() -> dict:
    """Mint one expiring, single-use opaque application handoff token with zero authority."""
    proof = request_proof.get()
    return result("probe_create_handle", handles.create(proof["owner"] if proof else None))


@mcp.tool()
def probe_consume_handle(application_handle: str) -> dict:
    """Pass the handle from probe_create_handle unchanged. Replay/expiry/other owner fails."""
    proof = request_proof.get()
    return result("probe_consume_handle", handles.consume(proof["owner"] if proof else None, application_handle))


class Guard:
    """Log safe request proof before MCP dispatch, including discovery/notifications.

    No URLs, arbitrary names, values, exception strings or bodies are logged.
    Fixed lifetime/request budgets bound even refused-traffic diagnostic output.
    """
    def __init__(self, app, clock=time.monotonic, emit=print, request_timeout=20,
                 header_observer=observe_headers,
                 zero_arg_tools=("probe_headers", "probe_create_handle"),
                 handle_tool="probe_consume_handle", extra_handle_tools=(), timed_diagnostics=False,
                 lifetime=MAX_LIFETIME):
        if type(lifetime) is not int or not 5 <= lifetime <= 3000:
            raise ValueError('invalid probe lifetime')
        self.lifetime = lifetime
        self.app, self.clock, self.emit = app, clock, emit
        self.request_timeout = request_timeout
        self.header_observer = header_observer
        self.zero_arg_tools = zero_arg_tools
        self.handle_tool = handle_tool
        if (not isinstance(extra_handle_tools, tuple) or len(extra_handle_tools) > 2
                or any(not isinstance(name, str) or not 1 <= len(name) <= 64 for name in extra_handle_tools)):
            raise ValueError('invalid fixed handle-tool allowlist')
        self.extra_handle_tools = extra_handle_tools
        self.timed_diagnostics = timed_diagnostics
        self.tool_emit = (lambda line: print(line, flush=True)) if emit is print else emit
        self.proof_version = 'fs-probe-v3' if timed_diagnostics else PROOF_VERSION
        self.instance = secrets.token_hex(6)
        self.started = clock()
        self.sequence = self.active = 0

    def valid_arguments(self, name, args):
        if name in self.zero_arg_tools:
            return args == {}
        if (self.handle_tool is not None and name == self.handle_tool) or name in self.extra_handle_tools:
            return (isinstance(args, dict) and set(args) == {"application_handle"}
                and isinstance(args["application_handle"], str)
                and HANDLE_RE.fullmatch(args["application_handle"]) is not None)
        return False

    def decode_request(self, body):
        return json.loads(body)

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            return await self.app(scope, receive, send)
        admitted = self.clock()
        if self.sequence >= MAX_REQUESTS or self.clock() - self.started >= self.lifetime:
            return await self.reject(send, 429)
        self.sequence += 1
        owner, observation = self.header_observer(scope.get("headers", []))
        raw_method = scope.get("method")
        # Only predefined labels leave the process: never echo arbitrary verbs,
        # URLs, paths, query strings, user-agent values, proxy headers or IPs.
        http_method = raw_method if raw_method in ("GET", "POST", "DELETE", "HEAD", "OPTIONS", "PUT", "PATCH") else "other"
        proof = {"server_instance": self.instance, "sequence": self.sequence,
                 "proof_version": self.proof_version, "http_method": http_method,
                 "path_is_mcp": scope.get("path") == "/mcp",
                 "query_present": bool(scope.get("query_string")),
                 "elapsed_ms": round(max(0, self.clock() - self.started) * 1000, 3),
                 "observation": observation}
        if self.timed_diagnostics:
            proof['timestamp_utc'] = timestamp_utc()
        self.emit("[FS-PROBE] HTTP " + json.dumps(proof, sort_keys=True))
        original_send = send
        response_started = False
        response_status = None
        stage = "header_bounds"
        async def safe_send(message):
            nonlocal response_started, response_status
            if message["type"] == "http.response.start":
                response_started = True
                response_status = message['status']
                self.emit("[FS-PROBE] STATUS " + json.dumps({
                    "server_instance": self.instance, "sequence": proof["sequence"],
                    "status": message["status"], "stage": stage,
                    "proof_version": self.proof_version,
                    **({'timestamp_utc': timestamp_utc(),
                        'elapsed_ms': round(max(0, self.clock() - self.started) * 1000, 3),
                        'request_elapsed_ms': round(max(0, self.clock() - admitted) * 1000, 3)}
                       if self.timed_diagnostics else {})}, sort_keys=True))
            await original_send(message)
            if (self.timed_diagnostics and message['type'] == 'http.response.body'
                    and not message.get('more_body', False)):
                now = self.clock()
                self.emit('[FS-PROBE] COMPLETE ' + json.dumps({
                    'server_instance': self.instance, 'sequence': proof['sequence'],
                    'proof_version': self.proof_version, 'status': response_status,
                    'timestamp_utc': timestamp_utc(),
                    'elapsed_ms': round(max(0, now - self.started) * 1000, 3),
                    'request_elapsed_ms': round(max(0, now - admitted) * 1000, 3)}, sort_keys=True))
        send = safe_send
        headers = scope.get("headers", [])
        if len(headers) > 64 or sum(len(k) + len(v) for k, v in headers) > MAX_HEADERS:
            return await self.reject(send, 431)
        stage = "endpoint"
        if scope.get("query_string") or scope.get("path") != "/mcp":
            return await self.reject(send, 400)
        stage = "dummy_bearer"
        if not owner:
            return await self.reject(send, 401)
        stage = "dummy_custom_headers"
        if not all(v["matches_dummy"] for v in observation["custom"].values()):
            return await self.reject(send, 401)
        stage = "admission"
        if self.active >= MAX_ACTIVE:
            return await self.reject(send, 503)
        self.active += 1
        token = request_proof.set({**proof, "owner": owner, 'tool_emit': self.tool_emit,
            'timed_diagnostics': self.timed_diagnostics, 'clock': self.clock,
            'started': self.started, 'admitted': admitted})
        try:
            async with asyncio.timeout(self.request_timeout):
                stage = "body_bounds"
                body = bytearray()
                while True:
                    message = await receive()
                    if message["type"] == "http.disconnect":
                        return
                    chunk = message.get("body", b"")
                    if len(body) + len(chunk) > MAX_BODY:
                        return await self.reject(send, 413)
                    body.extend(chunk)
                    if not message.get("more_body", False):
                        break
                delivered = False
                if body:
                    stage = "request_shape"
                    try:
                        request = self.decode_request(body)
                        if not isinstance(request, dict):
                            raise ValueError()
                        if request.get("method") == "tools/call":
                            params = request.get("params", {})
                            name = params.get("name")
                            args = params.get("arguments", {})
                            valid = self.valid_arguments(name, args)
                            if not valid:
                                raise ValueError()
                    except (ValueError, TypeError, AttributeError, RecursionError):
                        return await self.reject(send, 400)
                    method = request.get("method")
                    safe_method = method if method in ("initialize", "notifications/initialized", "tools/list",
                        "tools/call", "discover", "ping") else "other"
                    self.emit("[FS-PROBE] DISPATCH " + json.dumps({"sequence": proof["sequence"],
                        "server_instance": self.instance, "method": safe_method}, sort_keys=True))
                async def replay():
                    nonlocal delivered
                    if not delivered:
                        delivered = True
                        return {"type": "http.request", "body": bytes(body), "more_body": False}
                    return await receive()
                stage = "mcp"
                await self.app(scope, replay, send)
        except TimeoutError:
            stage = "timeout"
            if not response_started:
                await self.reject(send, 408)
        except Exception:
            # Do not format exceptions: parsers/SDKs can embed caller-controlled values.
            stage = "internal_error"
            if not response_started:
                await self.reject(send, 500)
        finally:
            request_proof.reset(token)
            self.active -= 1

    @staticmethod
    async def reject(send, status):
        await send({"type": "http.response.start", "status": status,
                    "headers": [(b"content-type", b"application/json")]})
        await send({"type": "http.response.body", "body": b'{"error":"probe.request_rejected"}'})


def make_app(allow_random_tunnel_host=False):
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow_random_tunnel_host else TransportSecuritySettings(
        enable_dns_rebinding_protection=True,
        allowed_hosts=["127.0.0.1:*", "localhost:*", "[::1]:*"],
        allowed_origins=["http://127.0.0.1:*", "http://localhost:*", "http://[::1]:*"])
    return Guard(mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security))


async def main():
    # Disable third-party logs completely; only the bounded safe proof channel remains.
    logging.disable(logging.CRITICAL)
    port = int(os.environ.get("OPTIC_FS_PROBE_PORT", "8001"))
    allow = os.environ.get("OPTIC_FS_PROBE_RANDOM_TUNNEL", "0") == "1"
    print(f"Authority NONE | dummy-only | http://127.0.0.1:{port}/mcp | lifetime 25 min | {PROOF_VERSION}", flush=True)
    print(f"Random tunnel host experiment switch: {allow}", flush=True)
    server = uvicorn.Server(uvicorn.Config(make_app(allow), host="127.0.0.1", port=port,
        access_log=False, log_config=None, limit_concurrency=32, timeout_keep_alive=5,
        h11_max_incomplete_event_size=MAX_HEADERS, timeout_graceful_shutdown=5))
    async def stop():
        await asyncio.sleep(MAX_LIFETIME)
        server.should_exit = True
    timer = asyncio.create_task(stop())
    try:
        await server.serve()
    finally:
        timer.cancel()


if __name__ == "__main__":
    asyncio.run(main())
