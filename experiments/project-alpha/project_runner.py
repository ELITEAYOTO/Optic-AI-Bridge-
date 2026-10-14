"""Generated persistent-output project playground, never an existing user project."""
import argparse
import asyncio
import json
import logging
from pathlib import Path
import sys
import tempfile
import threading

HERE = Path(__file__).resolve().parent
for directory in ('real-link-admission', 'credential-lifecycle', 'session-resolver'):
    sys.path.insert(0, str(HERE.parent / directory))

import uvicorn
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from real_smoke import structured
from lifecycle_core import Registry, TOKENS
from http_adapter import local_stop
from host_runner import console
from proof_channel import ProofChannel
from project_gate import ProjectGate
from project_http import make_project_app
from execution_gate import ExecutionGate
from node_profiles import configure_node
from viewer_snapshot import write_viewer_ready


def prepare_project(output_root):
    output = Path(output_root).resolve(strict=True)
    if not output.is_dir():
        raise ValueError('project.output_missing')
    run = Path(tempfile.mkdtemp(prefix='alpha-', dir=output))
    project, journal = run / 'project', run / 'journal'
    project.mkdir()
    journal.mkdir()
    for leaf in ('src', 'src/assets', 'src/main', 'src/main/java', 'src/main/java/example', 'src/main/resources'):
        (project / leaf).mkdir()
    (project / 'src' / 'README.txt').write_text(
        'Public generated Optic alpha playground. Create index.html, style.css and app.js here.\n'
        'This run has file editing only; no build/process/network/delete/Git tools.\n', encoding='utf-8')
    return run, project, journal


async def run(args):
    if sys.platform != 'win32':
        raise ValueError('project.windows_required')
    binary = Path(args.binary).resolve(strict=True)
    if not binary.is_file():
        raise ValueError('project.binary_missing')
    root, project, journal = prepare_project(args.output_root)
    profiles, process_arguments = {}, []
    if args.node:
        if not (binary.parent / 'optic-bridge-isolation-launcher.exe').is_file():
            raise ValueError('project.isolation_helper_required')
        profiles, process_arguments = configure_node(project, args.node)
        (project / 'src/README.txt').write_text(
            'Generated Node alpha. Edit index.html/style.css and app.js for the browser.\n'
            'Node can read model.js/app.js/tests.js only. check validates JavaScript syntax; '
            'test runs tests.js with relative model.js imports. No build output, dependencies or server.\n', encoding='utf-8')
    channel = ProofChannel()
    try:
        params = StdioServerParameters(command=str(binary), args=[str(project),
            '--allow-write-scope', 'prefix:src', '--mutation-state-dir', str(journal),
            '--session-lifetime-seconds', str(args.lifetime), *process_arguments])
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as upstream:
                async with asyncio.timeout(20):
                    await upstream.initialize()
                    names = {item.name for item in (await upstream.list_tools()).tools}
                    if not {'session_info', 'fs_list', 'fs_read', 'fs_write', 'fs_apply_patch', 'session_cancel'} <= names:
                        raise ValueError('project.upstream_surface')
                    # Exact package must provide observed read versions, not guessed hashes.
                    baseline = structured(await upstream.call_tool('fs_read',
                        {'path': 'src/README.txt', 'max_bytes': 8192}))
                    if not isinstance(baseline.get('content_version'), str) or len(baseline['content_version']) != 64:
                        raise ValueError('project.versioned_binary_required')
                state = Registry()
                identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                async def forward(name, arguments):
                    return structured(await upstream.call_tool(name, arguments))
                gate = ProjectGate(state, identity, forward)
                execution = ExecutionGate(state, identity, forward, profiles) if profiles else None
                app = make_project_app(gate, state, emit=channel.emit,
                    allow_random_tunnel=args.allow_random_tunnel_host, lifetime=args.lifetime,
                    execution=execution, compact_results=bool(profiles))
                service = uvicorn.Server(uvicorn.Config(app, host='127.0.0.1', port=args.port,
                    log_config=None, access_log=False, limit_concurrency=32, timeout_keep_alive=5,
                    timeout_graceful_shutdown=5, h11_max_incomplete_event_size=16384))
                stopped = asyncio.Event()
                loop = asyncio.get_running_loop()
                threading.Thread(target=console, args=(state, loop, stopped, channel), daemon=True).start()
                async def watch():
                    if args.lifetime > 120:
                        try:
                            async with asyncio.timeout(args.lifetime - 120):
                                await stopped.wait()
                        except TimeoutError:
                            channel.emit('[LOCAL] ' + json.dumps({'operation': 'expiry-warning',
                                'remaining_seconds': 120, 'project_saved_after_stop': True}, sort_keys=True))
                    try:
                        async with asyncio.timeout(min(args.lifetime, 120)):
                            await stopped.wait()
                    except TimeoutError:
                        pass
                    gate.stopped = True
                    service.should_exit = True
                watcher = asyncio.create_task(watch())
                mode = 'editing and isolated Node check/test' if profiles else 'file editing only'
                channel.emit(f'GENERATED PROJECT ALPHA | http://127.0.0.1:{args.port}/mcp | {mode}')
                if profiles:
                    channel.emit('Local Node: ' + profiles['check']['executable'])
                    channel.emit('Fixed profiles: check / test. Reads: model.js / app.js / tests.js. Network and process writes denied.')
                channel.emit('Public dummy auth. Shared connection. Local: rotate-a / revoke-a / stop / proof-stats.')
                channel.emit('Session limit: ' + str(args.lifetime) + ' seconds; project_info reports remaining time. No automatic renewal.')
                channel.emit('Project saved under output/' + root.name + '/project/src after stop. No existing project is accepted.')
                try:
                    await service.serve()
                finally:
                    watcher.cancel()
                    outcome = await local_stop(gate, upstream)
                    channel.emit('[LOCAL] ' + json.dumps({'operation': 'stop', **outcome}, sort_keys=True))
                    if profiles and outcome.get('ok'):
                        try:
                            ready = write_viewer_ready(root)
                        except (OSError, ValueError):
                            ready = {'viewer_ready': False, 'code': 'viewer.sources_not_ready'}
                        channel.emit('[LOCAL] ' + json.dumps({'operation': 'viewer-snapshot', **ready}, sort_keys=True))
    finally:
        channel.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--node', help='Locally selected installed node.exe; optional generated check/test mode')
    parser.add_argument('--output-root', required=True)
    parser.add_argument('--port', type=int, default=8009)
    parser.add_argument('--lifetime', type=int, default=3000)
    parser.add_argument('--allow-random-tunnel-host', action='store_true')
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535 or not 5 <= args.lifetime <= 3000:
        raise SystemExit('project.invalid_bounds')
    logging.disable(logging.CRITICAL)
    try:
        asyncio.run(run(args))
    except KeyboardInterrupt:
        pass
    except Exception:
        raise SystemExit('project.runner_failed') from None


if __name__ == '__main__':
    main()
