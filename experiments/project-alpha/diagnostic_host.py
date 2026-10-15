"""Generated-data-only MCP target. Never connects to the Rust/project server."""
import argparse
import asyncio
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import secrets
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'francestudent-probe'))
sys.path.insert(0, str(HERE.parent / 'session-resolver'))
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
from server import Guard, result, request_proof
from proof_channel import ProofChannel
import uvicorn

SIZES = (128, 1024, 4096, 8192)
DELAYS = (0, 1, 3, 10)


def make_diagnostic_app(emit, allow_random_tunnel=False, lifetime=1200):
    run_id = secrets.token_hex(8)
    build_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    mcp = MCPServer('Optic-Connection-Diagnostic', instructions=(
        'Synthetic public data only. Exactly one tool diagnostic_sample. '
        'No project, files, process, network effects or permission controls. '
        'Use size_bytes128/1024/4096/8192 and delay_seconds0/1/3/10.'))

    @mcp.tool()
    async def diagnostic_sample(size_bytes: int = 128, delay_seconds: int = 0) -> dict:
        """Return verifiable synthetic ASCII after a fixed bounded delay."""
        if type(size_bytes) is not int or size_bytes not in SIZES or type(delay_seconds) is not int or delay_seconds not in DELAYS:
            raise ValueError('diagnostic.invalid_arguments')
        proof = request_proof.get()
        emit('[DIAGNOSTIC] BEGIN ' + json.dumps({'diagnostic_run': run_id,
            'server_instance': proof['server_instance'], 'request_sequence': proof['sequence'],
            'size_bytes': size_bytes, 'delay_seconds': delay_seconds,
            'timestamp_utc': datetime.now(timezone.utc).isoformat(timespec='milliseconds')}, sort_keys=True))
        await asyncio.sleep(delay_seconds)
        text = 'd' * size_bytes
        return result('diagnostic_sample', {'ok': True, 'authority': 'none',
            'diagnostic_run': run_id, 'build_sha256': build_sha256,
            'size_bytes': size_bytes, 'delay_seconds': delay_seconds,
            'sha256': hashlib.sha256(text.encode('ascii')).hexdigest(), 'text': text})

    class DiagnosticGuard(Guard):
        def valid_arguments(self, name, args):
            return (name == 'diagnostic_sample' and type(args) is dict
                and not (set(args) - {'size_bytes', 'delay_seconds'})
                and type(args.get('size_bytes', 128)) is int and args.get('size_bytes', 128) in SIZES
                and type(args.get('delay_seconds', 0)) is int and args.get('delay_seconds', 0) in DELAYS)

    sdk = mcp.streamable_http_app(stateless_http=True, json_response=True, max_request_body_size=65536,
        transport_security=TransportSecuritySettings(enable_dns_rebinding_protection=False)
            if allow_random_tunnel else TransportSecuritySettings(allowed_hosts=['127.0.0.1:*', 'localhost:*']))
    return DiagnosticGuard(sdk, emit=emit, zero_arg_tools=(), handle_tool=None,
                           timed_diagnostics=True, lifetime=lifetime)


async def serve(port, allow_random_tunnel, lifetime):
    channel = ProofChannel()
    app = make_diagnostic_app(channel.emit, allow_random_tunnel, lifetime)
    service = uvicorn.Server(uvicorn.Config(app, host='127.0.0.1', port=port,
        log_config=None, access_log=False, limit_concurrency=8, timeout_graceful_shutdown=5))
    async def expire():
        await asyncio.sleep(lifetime)
        service.should_exit = True
    expiry = asyncio.create_task(expire())
    channel.emit('DIAGNOSTIC ONLY | no project authority | port ' + str(port))
    try:
        await service.serve()
    finally:
        expiry.cancel()
        await asyncio.gather(expiry, return_exceptions=True)
        channel.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--port', type=int, default=8010)
    parser.add_argument('--lifetime', type=int, default=1200)
    parser.add_argument('--allow-random-tunnel-host', action='store_true')
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535 or not 5 <= args.lifetime <= 3000:
        parser.error('diagnostic.invalid_bounds')
    try:
        asyncio.run(serve(args.port, args.allow_random_tunnel_host, args.lifetime))
    except KeyboardInterrupt:
        pass
