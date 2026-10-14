"""Real Rust project create/read/edit proof; generated workspace only."""
import asyncio
import base64
from pathlib import Path
import sys
import tempfile

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from mcp.shared.exceptions import MCPError

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'real-link-admission'))
from real_smoke import structured


async def run(binary):
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='optic-project-alpha-proof-') as directory:
        parent = Path(directory)
        workspace, state = parent / 'project', parent / 'journal'
        workspace.mkdir()
        state.mkdir()
        (workspace / 'src').mkdir()
        sentinel = b'PUBLIC OUTSIDE WRITE SCOPE\n'
        (workspace / 'README.txt').write_bytes(sentinel)
        params = StdioServerParameters(command=str(binary), args=[str(workspace),
            '--allow-write-scope', 'prefix:src', '--mutation-state-dir', str(state)])
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as client:
                await client.initialize()
                tools = {item.name for item in (await client.list_tools()).tools}
                assert {'fs_read', 'fs_list', 'fs_write', 'fs_apply_patch', 'session_cancel'} <= tools
                session = structured(await client.call_tool('session_info', {}))
                assert 'file_write' in session['capabilities']
                assert not {'file_delete', 'process_run', 'git_integrate'}.intersection(session['capabilities'])
                first = b'<!doctype html><title>Optic Alpha</title><h1>Hello</h1>\n'
                improved = first.replace(b'Hello', b'Hello from a real project')
                def request(path, content, expected):
                    return {'path': path, 'content_base64': base64.b64encode(content).decode('ascii'),
                            'expected': expected}
                created = structured(await client.call_tool('fs_write',
                    request('src/index.html', first, {'kind': 'absent'})))
                assert (workspace / 'src' / 'index.html').read_bytes() == first
                observed = structured(await client.call_tool('fs_read',
                    {'path': 'src/index.html', 'offset': 0, 'max_bytes': 1024}))
                assert base64.b64decode(observed['data'], validate=True) == first
                replaced = structured(await client.call_tool('fs_write', request('src/index.html',
                    improved, {'kind': 'content', 'version_hex': created['content_version']})))
                assert replaced['content_version'] != created['content_version']
                for denied, code in ((request('src/index.html', first, {'kind': 'absent'}), 'optic.precondition_failed'),
                               (request('src/index.html', first,
                                   {'kind': 'content', 'version_hex': created['content_version']}), 'optic.precondition_failed'),
                               (request('README.txt', b'changed', {'kind': 'absent'}), 'optic.policy_denied'),
                               (request('../outside.txt', b'changed', {'kind': 'absent'}), 'optic.invalid_workspace_path')):
                    try:
                        rejected = await client.call_tool('fs_write', denied)
                    except MCPError as error:
                        assert error.message == code
                    else:
                        assert rejected.is_error
                assert (workspace / 'src' / 'index.html').read_bytes() == improved
                assert (workspace / 'README.txt').read_bytes() == sentinel
                assert not (parent / 'outside.txt').exists()
                assert not (await client.call_tool('session_cancel', {})).is_error
                try:
                    rejected = await client.call_tool('fs_write', request('src/new.html', first,
                        {'kind': 'absent'}))
                except MCPError as error:
                    assert error.message == 'optic.session_inactive'
                else:
                    assert rejected.is_error
                assert not (workspace / 'src' / 'new.html').exists()
    print('PASS: real Rust mini-project create/read/edit; stale/absent/out-of-scope/traversal/revoked writes denied.')
    print('Generated disposable project only. No FS live editing or process execution claim.')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('one test binary required')
    asyncio.run(asyncio.wait_for(run(sys.argv[1]), timeout=60))
