"""Manual generated-fixture runner; public dummy credentials only, never user paths."""
import argparse
import asyncio
import json
import logging
from pathlib import Path
import sys
import tempfile
import threading

import uvicorn
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from real_smoke import structured
from lifecycle_core import Registry, TOKENS
from forward_core import ForwardGate
from http_adapter import make_app, local_stop

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'session-resolver'))
from proof_channel import ProofChannel


def console(state, loop, stop, channel):
    # Exhausting diagnostics/control commands must never disable local stop.
    commands = 0
    while True:
        line = sys.stdin.readline(65)
        command = line.strip()
        if not line or len(line) > 64 or not line.endswith('\n') or command == 'stop':
            try:
                loop.call_soon_threadsafe(stop.set)
            except RuntimeError:
                pass  # The service already closed its event loop.
            return
        if commands >= 16:
            channel.emit('[LOCAL] command_budget_exhausted; stop remains available')
            continue
        commands += 1
        if command in ('rotate-a', 'revoke-a'):
            operation = 'rotate' if command == 'rotate-a' else 'revoke'
            payload = state.control(operation, 'A')
            channel.emit('[LOCAL] ' + json.dumps(payload, sort_keys=True))
        elif command == 'proof-stats':
            channel.emit('[LOCAL] ' + json.dumps(channel.snapshot(), sort_keys=True))
        else:
            channel.emit('[LOCAL] command_refused')


async def run(args):
    binary = Path(args.binary).resolve(strict=True)
    if not binary.is_file():
        raise ValueError('link.binary_missing')
    channel = ProofChannel()
    try:
        with tempfile.TemporaryDirectory(prefix='optic-manual-generated-link-') as directory:
            root = Path(directory)
            (root / 'public').mkdir()
            (root / 'public' / 'hello.txt').write_bytes(b'PUBLIC_GENERATED_OPTIC_HOST_FIXTURE\n')
            async with stdio_client(StdioServerParameters(command=str(binary), args=[str(root)])) as (read, write):
                async with ClientSession(read, write) as upstream:
                    async with asyncio.timeout(20):
                        await upstream.initialize()
                        names = {item.name for item in (await upstream.list_tools()).tools}
                        if not {'session_info', 'fs_list', 'fs_read', 'session_cancel'} <= names:
                            raise ValueError('link.upstream_surface')
                    state = Registry()
                    identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                    async def forward(name, arguments):
                        return structured(await upstream.call_tool(name, arguments))
                    gate = ForwardGate(state, identity, forward)
                    app = make_app(gate, state, emit=channel.emit,
                        allow_random_tunnel=args.allow_random_tunnel_host)
                    service = uvicorn.Server(uvicorn.Config(app, host='127.0.0.1', port=args.port,
                        log_config=None, access_log=False, limit_concurrency=32,
                        timeout_keep_alive=5, timeout_graceful_shutdown=5,
                        h11_max_incomplete_event_size=16384))
                    stopped = asyncio.Event()
                    loop = asyncio.get_running_loop()
                    threading.Thread(target=console, args=(state, loop, stopped, channel), daemon=True).start()
                    async def watch():
                        try:
                            async with asyncio.timeout(args.lifetime):
                                await stopped.wait()
                        except TimeoutError:
                            pass
                        gate.stopped = True
                        service.should_exit = True
                    watcher = asyncio.create_task(watch())
                    channel.emit(f'GENERATED FIXTURE ONLY | fixed-read-host-v1 | http://127.0.0.1:{args.port}/mcp')
                    channel.emit('Public dummy auth only. Shared single session. Local: rotate-a / revoke-a / stop / proof-stats.')
                    channel.emit('Random tunnel host: ' + str(args.allow_random_tunnel_host))
                    try:
                        await service.serve()
                    finally:
                        watcher.cancel()
                        outcome = await local_stop(gate, upstream)
                        channel.emit('[LOCAL] ' + json.dumps({'operation': 'stop', **outcome}, sort_keys=True))
    finally:
        channel.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--port', type=int, default=8008)
    parser.add_argument('--lifetime', type=int, default=1500)
    parser.add_argument('--allow-random-tunnel-host', action='store_true')
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535 or not 5 <= args.lifetime <= 1500:
        raise SystemExit('link.invalid_bounds')
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(run(args))
    except KeyboardInterrupt:
        pass
    except Exception:
        raise SystemExit('link.runner_failed') from None


if __name__ == '__main__':
    main()
