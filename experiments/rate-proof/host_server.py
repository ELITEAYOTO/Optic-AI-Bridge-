"""One harmless tool; fixed local-only hold/release for reproducible host429."""
import asyncio
import json
import logging
import os
from pathlib import Path
import sys
import threading

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'francestudent-probe'))
from mcp.server import MCPServer
from mcp.server.transport_security import TransportSecuritySettings
import uvicorn
from server import Guard, result, MAX_BODY, MAX_HEADERS, MAX_LIFETIME
from rate_core import Bucket, RateGate


class HeldBucket(Bucket):
    held = False
    def take(self):
        return False if self.held else super().take()


def make_app(allow=False):
    mcp = MCPServer('Optic-Synthetic-Rate-Probe', instructions=(
        'Only rate_probe: harmless metadata, no credentials or project authority.'))
    @mcp.tool()
    def rate_probe() -> dict:
        """Report a real invocation marker, authority none and safe header bits."""
        return result('rate_probe', {'ok': True, 'authority': 'none'})
    security = TransportSecuritySettings(enable_dns_rebinding_protection=False) if allow else TransportSecuritySettings(
        allowed_hosts=['127.0.0.1:*', 'localhost:*'],
        allowed_origins=['http://127.0.0.1:*', 'http://localhost:*'])
    upstream = mcp.streamable_http_app(stateless_http=True, json_response=True,
        max_request_body_size=MAX_BODY, transport_security=security)
    guard = Guard(upstream, zero_arg_tools=('rate_probe',), handle_tool=None)
    gate = RateGate(guard)
    gate.bucket = HeldBucket()
    return gate, guard


async def run():
    gate, guard = make_app(os.environ.get('OPTIC_RATE_TUNNEL', '0') == '1')
    loop = asyncio.get_running_loop()
    budget = 32
    async def observed(scope, receive, send):
        nonlocal budget
        async def safe_send(message):
            nonlocal budget
            if message['type'] == 'http.response.start' and message['status'] == 429 and budget:
                budget -= 1
                print('[RATE] ' + json.dumps({'server_instance': guard.instance,
                    'status': 429, 'stage': 'global_admission', 'remaining_log_budget': budget}), flush=True)
            await send(message)
        await gate(scope, receive, safe_send)
    def control(command):
        if command == 'hold-rate':
            gate.bucket.held = True
        elif command == 'release-rate':
            gate.bucket.held = False
            # Explicit synthetic recovery control, not natural refill evidence.
            gate.bucket.tokens = gate.bucket.capacity
            gate.bucket.last = gate.bucket.clock()
        else:
            return
        print('[LOCAL] ' + json.dumps({'operation': command, 'held': gate.bucket.held,
            'server_instance': guard.instance, 'authority': 'none'}), flush=True)
    def console():
        for _ in range(16):
            line = sys.stdin.readline(65)
            if not line or len(line) > 64:
                return
            command = line.strip()
            if command in ('hold-rate', 'release-rate'):
                loop.call_soon_threadsafe(control, command)
            else:
                print('[LOCAL] command_refused', flush=True)
    threading.Thread(target=console, daemon=True).start()
    port = int(os.environ.get('OPTIC_RATE_PORT', '8005'))
    print(f'Authority NONE | synthetic-rate-host-v1 | http://127.0.0.1:{port}/mcp | lifetime25min', flush=True)
    print('Local only: hold-rate / release-rate. Burst8/refill2 per second; no persistent state.', flush=True)
    service = uvicorn.Server(uvicorn.Config(observed, host='127.0.0.1', port=port,
        access_log=False, log_config=None, limit_concurrency=32, timeout_keep_alive=5,
        h11_max_incomplete_event_size=MAX_HEADERS, timeout_graceful_shutdown=5))
    async def stop():
        await asyncio.sleep(MAX_LIFETIME)
        service.should_exit = True
    timer = asyncio.create_task(stop())
    try:
        await service.serve()
    finally:
        timer.cancel()


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(run())
    except KeyboardInterrupt:
        pass
