"""Actual HTTP -> typed Rust generated-project editing and failure proof."""
import asyncio
import base64
import json
import logging
from pathlib import Path
import socket
import sys
import tempfile

HERE = Path(__file__).resolve().parent
for directory in ('real-link-admission', 'credential-lifecycle'):
    sys.path.insert(0, str(HERE.parent / directory))

import uvicorn
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from project_results import upstream as upstream_structured
from project_results import structured
from lifecycle_core import Registry, TOKENS
from http_smoke import client, denied
from http_adapter import local_stop
from forward_core import ForwardError
from project_gate import ProjectGate
from project_http import make_project_app
from project_schema import TOOLS


async def replacement_permissions(binary):
    # The convenience adapter must not combine read-only and write-only grants
    # into broader authority. Exercise the actual core, not an adapter stub.
    for allow_read, allow_write in ((False, True), (True, False)):
        with tempfile.TemporaryDirectory(prefix='optic-replace-permissions-') as directory:
            base = Path(directory)
            project, journal = base / 'project', base / 'journal'
            (project / 'src').mkdir(parents=True)
            journal.mkdir()
            target = project / 'src' / 'only.txt'
            target.write_bytes(b'old')
            config = base / 'operator.json'
            config.write_text(json.dumps({'version': 1, 'policy_epoch': 1,
                'workspace': str(project),
                'permissions': {'files': {'read': allow_read, 'write': allow_write}},
                'mutation': {'write_scopes': ['prefix:src'], 'state_dir': str(journal)}}), encoding='utf-8')
            params = StdioServerParameters(command=str(Path(binary).resolve(strict=True)),
                args=['--config', str(config)])
            async with stdio_client(params) as (read, write):
                async with ClientSession(read, write) as core:
                    await core.initialize()
                    state = Registry()
                    owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                    names = {item.name for item in (await core.list_tools()).tools}
                    if not allow_write:
                        assert 'fs_apply_patch' not in names
                    calls = []
                    async def forward(name, args):
                        calls.append(name)
                        return upstream_structured(await core.call_tool(name, args))
                    gate = ProjectGate(state, owner, forward)
                    # Content versions are opaque core receipts, not a caller's
                    # recomputed raw SHA256. No read is attempted when denied.
                    observed = (upstream_structured(await core.call_tool('fs_read', {
                        'path': 'src/only.txt', 'offset': 0, 'max_bytes': 32}))['content_version']
                        if allow_read else '0' * 64)
                    try:
                        await gate.call(owner, 'optic_fs_replace', {'path': 'src/only.txt',
                            'expected_version': observed,
                            'old_text': 'old', 'new_text': 'new'})
                    except ForwardError as error:
                        # A disabled write surface may be rejected by the SDK
                        # before the core handler, with its generic redacted
                        # error. Retain conservative uncertainty; never replay.
                        assert str(error) in ({'optic.policy_denied'} if not allow_read else
                                              {'optic.policy_denied', 'project.outcome_unknown'}), error
                    else:
                        raise AssertionError('independent replacement permissions bypassed')
                    assert calls == (['fs_read'] if not allow_read else ['fs_read', 'fs_apply_patch']), calls
                    assert target.read_bytes() == b'old'
                    await local_stop(gate, core)


async def run(binary, max_file_bytes=8192):
    from project_schema import file_limit
    file_limit(max_file_bytes)
    logs = []
    with tempfile.TemporaryDirectory(prefix='optic-alpha-http-proof-') as directory:
        parent = Path(directory)
        project, journal = parent / 'project', parent / 'journal'
        project.mkdir()
        journal.mkdir()
        (project / 'src').mkdir()
        (project / 'src' / 'README.txt').write_bytes(b'PUBLIC ALPHA PROJECT\n')
        params = StdioServerParameters(command=str(Path(binary).resolve(strict=True)), args=[str(project),
            '--allow-write-scope', 'prefix:src', '--mutation-state-dir', str(journal)])
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as upstream:
                await upstream.initialize()
                state = Registry()
                identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                race_once = [False]
                async def forward(name, args):
                    if name == 'fs_apply_patch' and args.get('path') == 'src/race.txt' and race_once[0]:
                        race_once[0] = False
                        # Simulate an external editor only inside this disposable fixture.
                        (project / 'src' / 'race.txt').write_bytes(b'EXTERNAL EDIT')
                    return upstream_structured(await upstream.call_tool(name, args), max_file_bytes)
                gate = ProjectGate(state, identity, forward, max_file_bytes=max_file_bytes)
                app = make_project_app(gate, state, emit=logs.append)
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen(32)
                url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
                service = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False,
                                                       timeout_graceful_shutdown=3))
                task = asyncio.create_task(service.serve(sockets=[listener]))
                try:
                    async with asyncio.timeout(10):
                        while not service.started:
                            if task.done():
                                await task
                                raise AssertionError('startup failed')
                            await asyncio.sleep(.01)
                    async with client(url, TOKENS['A'][0]) as session:
                        assert {tool.name for tool in (await session.list_tools()).tools} == set(TOOLS)
                        initial = structured(await session.call_tool('optic_project_info', {}))
                        assert initial['limits']['file_bytes'] == max_file_bytes
                        listing = structured(await session.call_tool('optic_fs_list', {}))
                        assert listing['upstream']['entries'][0]['path'] == 'src/README.txt'
                        baseline = structured(await session.call_tool('optic_fs_read', {'path': 'src/README.txt'}))
                        assert len(baseline['upstream']['content_version']) == 64
                        version = baseline['upstream']['content_version']
                        created = structured(await session.call_tool('optic_fs_write', {
                            'path': 'src/index.html', 'content': '<h1>Public Alpha</h1>\n', 'expected_kind': 'absent'}))
                        if sys.platform != 'win32':
                            assert not created['ok'] and created['code'] == 'optic.mutation_platform_unsupported'
                            assert not (project / 'src' / 'index.html').exists()
                        else:
                            assert created['ok']
                            observed = structured(await session.call_tool('optic_fs_read', {'path': 'src/index.html'}))
                            version = observed['upstream']['content_version']
                            assert version == created['upstream']['content_version']
                            assert observed['upstream']['text'] == '<h1>Public Alpha</h1>\n'
                            found = structured(await session.call_tool('optic_fs_search', {
                                'path': 'src/index.html', 'text': 'Public'}))
                            assert found['upstream']['matches'][0]['line'] == 1
                            patched = structured(await session.call_tool('optic_fs_patch', {
                                'path': 'src/index.html', 'expected_version': version,
                                'offset': 4, 'remove_bytes': 6, 'insert': 'Edited'}))
                            assert patched['ok']
                            stale = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/index.html', 'content': 'stale', 'expected_kind': 'content',
                                'expected_version': version}))
                            assert not stale['ok'] and stale['code'] == 'optic.precondition_failed'
                            assert (project / 'src' / 'index.html').read_text() == '<h1>Edited Alpha</h1>\n'
                            ambiguous = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/index.html', 'content': 'duplicate', 'expected_kind': 'absent'}))
                            assert not ambiguous['ok'] and ambiguous['code'] == 'optic.precondition_failed'
                            partial = structured(await session.call_tool('optic_fs_read', {
                                'path': 'src/index.html', 'max_bytes': 2}))
                            assert partial['upstream']['content_version'] == patched['upstream']['content_version']
                            assert partial['upstream']['next_offset'] == 2
                            assert not partial['upstream']['eof']
                            long_text = 'a' * 1023 + 'é' + 'b' * (32768 - 1025 if max_file_bytes == 32768 else 1400)
                            long_write = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/paged.txt', 'content': long_text, 'expected_kind': 'absent'}))
                            assert long_write['ok']
                            offset, chunks = 0, []
                            while True:
                                page = structured(await session.call_tool('optic_fs_read', {
                                    'path': 'src/paged.txt', 'offset': offset, 'max_bytes': 8192}))
                                assert page.get('ok'), page
                                page = page['upstream']
                                assert page['content_version'] == long_write['upstream']['content_version']
                                assert 'data' not in page and len(page['text'].encode()) <= 1024
                                chunks.append(page['text'])
                                offset = page['next_offset']
                                if page['eof']:
                                    break
                            assert ''.join(chunks) == long_text
                            # Exact text replacement uses Rust's original version check,
                            # preserving UTF-8/CRLF and every byte outside the selected match.
                            source = 'é🙂\r\nold\r\ntail'
                            made = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/replace.txt', 'content': source, 'expected_kind': 'absent'}))
                            first_version = made['upstream']['content_version']
                            replaced = structured(await session.call_tool('optic_fs_replace', {
                                'path': 'src/replace.txt', 'expected_version': first_version,
                                'old_text': 'old', 'new_text': 'new €'}))
                            assert replaced['ok'], replaced
                            assert (project / 'src' / 'replace.txt').read_bytes() == 'é🙂\r\nnew €\r\ntail'.encode()
                            stale = structured(await session.call_tool('optic_fs_replace', {
                                'path': 'src/replace.txt', 'expected_version': first_version,
                                'old_text': 'new €', 'new_text': 'stale'}))
                            assert not stale['ok'] and stale['code'] == 'optic.precondition_failed', stale
                            duplicate = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/replace.txt', 'content': 'aaa', 'expected_kind': 'content',
                                'expected_version': replaced['upstream']['content_version']}))
                            rejected = structured(await session.call_tool('optic_fs_replace', {
                                'path': 'src/replace.txt', 'expected_version': duplicate['upstream']['content_version'],
                                'old_text': 'aa', 'new_text': 'Z'}))
                            assert not rejected['ok'] and rejected['code'] == 'project.match_ambiguous', rejected
                            assert (project / 'src' / 'replace.txt').read_bytes() == b'aaa'
                            raced = structured(await session.call_tool('optic_fs_write', {
                                'path': 'src/race.txt', 'content': 'old', 'expected_kind': 'absent'}))
                            race_once[0] = True
                            conflict = structured(await session.call_tool('optic_fs_replace', {
                                'path': 'src/race.txt', 'expected_version': raced['upstream']['content_version'],
                                'old_text': 'old', 'new_text': 'new'}))
                            assert not conflict['ok'] and conflict['code'] == 'optic.precondition_failed', conflict
                            assert (project / 'src' / 'race.txt').read_bytes() == b'EXTERNAL EDIT'
                            if max_file_bytes == 32768:
                                # Exercise an actual native mutation beyond the old 8KiB bound.
                                changed = structured(await session.call_tool('optic_fs_patch', {
                                    'path': 'src/paged.txt', 'expected_version': long_write['upstream']['content_version'],
                                    'offset': 10000, 'remove_bytes': 1, 'insert': 'Z'}))
                                assert changed['ok'], changed
                                stale = structured(await session.call_tool('optic_fs_patch', {
                                    'path': 'src/paged.txt', 'expected_version': long_write['upstream']['content_version'],
                                    'offset': 10000, 'remove_bytes': 1, 'insert': 'Q'}))
                                assert not stale['ok'] and stale['code'] == 'optic.precondition_failed', stale
                                tail = structured(await session.call_tool('optic_fs_read', {
                                    'path': 'src/paged.txt', 'offset': 10000, 'max_bytes': 1}))['upstream']
                                assert tail['text'] == 'Z' and tail['content_version'] == changed['upstream']['content_version']
                                literal = structured(await session.call_tool('optic_fs_replace', {
                                    'path': 'src/paged.txt', 'expected_version': changed['upstream']['content_version'],
                                    'old_text': 'Z', 'new_text': 'R'}))
                                assert literal['ok'], literal
                                assert (project / 'src' / 'paged.txt').read_bytes()[10000:10001] == b'R'


                    await denied(url, TOKENS['B'][0])
                    await denied(url, TOKENS['A'][0], 400, {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                        'params': {'name': 'optic_fs_write', 'arguments': {'path': '../CANARY', 'content': 'x', 'expected_kind': 'absent'}}})
                    state.control('rotate', 'A')
                    await denied(url, TOKENS['A'][0])
                    async with client(url, TOKENS['A'][1]) as session:
                        assert structured(await session.call_tool('optic_project_info', {}))['upstream'] == initial['upstream']
                    state.control('revoke', 'A')
                    await denied(url, TOKENS['A'][1])
                    assert (await local_stop(gate, upstream))['session_revoked']
                    capture = '\n'.join(logs)
                    for value in (str(project), 'CANARY', '<h1>', *(token.decode() for values in TOKENS.values() for token in values)):
                        assert value not in capture
                    assert all(json.loads(line.split(' TOOL ', 1)[1])['tool'] in TOOLS for line in logs if ' TOOL ' in line)
                finally:
                    gate.stopped = True
                    service.should_exit = True
                    await asyncio.wait_for(task, 10)
                    listener.close()
    await replacement_permissions(binary)
    print('ALPHA HTTP PASS: exact editing-tool surface, real versioned reads, Windows create/edit/patch/conflict proof or non-Windows refusal, rotation/reconnect/revoke/stop.')


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    asyncio.run(asyncio.wait_for(run(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 8192), timeout=90))
