"""Native Windows HTTP -> Rust -> isolated installed Node, disposable files only."""
import asyncio
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
from real_smoke import structured as upstream_structured
from project_results import structured
from http_smoke import client
from lifecycle_core import Registry, TOKENS
from project_gate import ProjectGate
from execution_gate import ExecutionGate, TASK_TOOLS
from project_http import make_project_app
from project_schema import TOOLS
from http_adapter import local_stop


async def finished(session, job):
    async with asyncio.timeout(15):
        while True:
            value = structured(await session.call_tool('optic_task_result', {'job_id': job}))
            assert value['ok'], value
            if value['upstream']['status'] != 'running':
                return value['upstream']
            await asyncio.sleep(.1)


async def run(binary, node):
    if sys.platform != 'win32':
        print('NODE HTTP SKIP: requires actual Windows AppContainer runtime.')
        return
    bridge = Path(binary).resolve(strict=True)
    executable = Path(node).resolve(strict=True)
    assert (bridge.parent / 'optic-bridge-isolation-launcher.exe').is_file()
    assert executable.name.lower() == 'node.exe'
    with tempfile.TemporaryDirectory(prefix='optic-node-http-') as directory:
        root = Path(directory)
        project, journal = root / 'project', root / 'journal'
        project.mkdir()
        journal.mkdir()
        (project / 'src').mkdir()
        logic, tests = project / 'src/logic.js', project / 'src/tests.js'
        ungranted = project / 'src/ungranted.txt'
        ungranted.write_text('PUBLIC_UNGRANTED_SENTINEL', encoding='utf-8')
        module_source, module_test = project / 'src/math.js', project / 'src/module-test.js'
        module_source.write_text('module.exports.add=(a,b)=>a+b;', encoding='utf-8')
        module_test.write_text("const assert=require('node:assert/strict'),fs=require('node:fs');"
            "assert.equal(fs.readFileSync(process.argv[2],'utf8'),'module.exports.add=(a,b)=>a+b;');"
            "console.log('DIRECT_GRANTED_BYTES_READ');const math=require('./math.js');"
            "assert.equal(math.add(2,3),5);assert.equal(fs.statSync(__dirname).isDirectory(),true);"
            "assert.throws(()=>fs.readdirSync(__dirname));assert.throws(()=>fs.readFileSync(process.argv[3]));"
            "assert.throws(()=>fs.writeFileSync(__dirname+'/forbidden-new.txt','x'));"
            "console.log('MODULE_TREE_PASS:LIST_READ_WRITE_DENIED');", encoding='utf-8')
        logic.write_text('function add(a,b){ return a-b; } add(2,3);', encoding='utf-8')
        tests.write_text("const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');"
            "const value=vm.runInNewContext(fs.readFileSync(process.argv[2],'utf8'));"
            "assert.equal(value,5);let denied=false;try{fs.readFileSync(process.argv[3]);}catch{denied=true;}"
            "assert.equal(denied,true);console.log('TEST_PASS:SIBLING_DENIED');", encoding='utf-8')
        def profile(args):
            return {'executable': str(executable), 'args': args, 'cwd': None, 'env_allowlist': [],
                    'network': False, 'timeout_ms': 10000, 'output_budget': 65536,
                    'memory_bytes': 268435456, 'process_count': 1}
        # No vm isolation claim: Windows AppContainer is the containment.
        # Direct module entry loading needs ancestor metadata beyond current
        # exact-file grants, measured as a separate negative characterization.
        check_driver = "const fs=require('node:fs'),vm=require('node:vm');new vm.Script(fs.readFileSync(process.argv[1],'utf8'));"
        test_driver = "const fs=require('node:fs'),vm=require('node:vm');vm.runInNewContext(fs.readFileSync(process.argv[1],'utf8'),{require,process,console});"
        profiles = {'check': profile(['-e', check_driver, str(logic)]),
                    'test': profile(['-e', test_driver, str(tests), str(logic), str(ungranted)]),
                    'module-flags': profile(['--preserve-symlinks-main', '--preserve-symlinks', '--check', str(logic)]),
                    'entry-test': profile(['--preserve-symlinks-main', '--preserve-symlinks', str(tests), str(logic), str(ungranted)]),
                    'module-tree': profile(['--preserve-symlinks-main', '--preserve-symlinks', str(module_test), str(module_source), str(ungranted)]),
                    'module-check': profile(['--check', str(logic)])}
        params = StdioServerParameters(command=str(bridge), args=[str(project),
            '--allow-write-scope', 'prefix:src', '--mutation-state-dir', str(journal),
            '--allow-executable', 'interpreter:' + str(executable),
            '--allow-isolated-node', str(executable),
            '--allow-process-read-file', str(executable), 'src/logic.js',
            '--allow-process-read-file', str(executable), 'src/tests.js',
            '--allow-process-read-file', str(executable), 'src/math.js',
            '--allow-process-read-file', str(executable), 'src/module-test.js'])
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as upstream:
                await upstream.initialize()
                async def forward(name, args):
                    return upstream_structured(await upstream.call_tool(name, args))
                state = Registry()
                identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
                files = ProjectGate(state, identity, forward)
                execution = ExecutionGate(state, identity, forward, profiles)
                logs = []
                app = make_project_app(files, state, emit=logs.append, execution=execution, compact_results=True)
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen(32)
                url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
                server = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False,
                                                      timeout_graceful_shutdown=3))
                service = asyncio.create_task(server.serve(sockets=[listener]))
                try:
                    async with asyncio.timeout(10):
                        while not server.started:
                            if service.done():
                                await service
                                raise AssertionError('startup failed')
                            await asyncio.sleep(.01)
                    async with client(url, TOKENS['A'][0]) as session:
                        assert {tool.name for tool in (await session.list_tools()).tools} == set(TOOLS + TASK_TOOLS)
                        failed = structured(await session.call_tool('optic_task_start', {'profile': 'test'}))
                        assert failed['ok'], failed
                        before = await finished(session, failed['upstream']['job_id'])
                        assert before['status'] == 'exited' and before['exit_code'] != 0
                        error = structured(await session.call_tool('optic_task_read',
                            {'job_id': failed['upstream']['job_id'], 'stream': 'stderr'}))
                        assert 'AssertionError' in error['upstream']['text'], error
                        snapshot = structured(await session.call_tool('optic_fs_read', {'path': 'src/logic.js'}))
                        assert snapshot['ok'] and snapshot['upstream']['eof']
                        fixed = structured(await session.call_tool('optic_fs_write', {
                            'path': 'src/logic.js', 'content': 'function add(a,b){ return a+b; } add(2,3);',
                            'expected_kind': 'content', 'expected_version': snapshot['upstream']['content_version']}))
                        assert fixed['ok'], fixed
                        for selected in ('check', 'test'):
                            started = structured(await session.call_tool('optic_task_start', {'profile': selected}))
                            assert started['ok'], started
                            result = await finished(session, started['upstream']['job_id'])
                            assert result['status'] == 'exited' and result['exit_code'] == 0, result
                            if selected == 'test':
                                output = structured(await session.call_tool('optic_task_read',
                                    {'job_id': started['upstream']['job_id']}))
                                assert 'TEST_PASS' in output['upstream']['text']
                        module = structured(await session.call_tool('optic_task_start', {'profile': 'module-check'}))
                        assert module['ok']
                        blocked = await finished(session, module['upstream']['job_id'])
                        assert blocked['status'] == 'exited'
                        if blocked['exit_code'] != 0:
                            details = structured(await session.call_tool('optic_task_read',
                                {'job_id': module['upstream']['job_id'], 'stream': 'stderr'}))
                            assert 'EPERM' in details['upstream']['text'] and 'lstat' in details['upstream']['text']
                        # Ancestor ACL baselines vary by host; characterize,
                        # never treat unrelated failure/timeout as success.
                        print('NODE_UNFLAGGED_ENTRY_EXIT', blocked['exit_code'])
                        flags = structured(await session.call_tool('optic_task_start', {'profile': 'module-flags'}))
                        assert flags['ok']
                        flag_result = await finished(session, flags['upstream']['job_id'])
                        assert flag_result['status'] == 'exited' and flag_result['exit_code'] == 0
                        entry = structured(await session.call_tool('optic_task_start', {'profile': 'entry-test'}))
                        assert entry['ok']
                        entry_result = await finished(session, entry['upstream']['job_id'])
                        assert entry_result['status'] == 'exited' and entry_result['exit_code'] == 0
                        entry_output = structured(await session.call_tool('optic_task_read',
                            {'job_id': entry['upstream']['job_id']}))
                        assert 'TEST_PASS:SIBLING_DENIED' in entry_output['upstream']['text']
                        tree = structured(await session.call_tool('optic_task_start', {'profile': 'module-tree'}))
                        assert tree['ok']
                        tree_result = await finished(session, tree['upstream']['job_id'])
                        tree_output = structured(await session.call_tool('optic_task_read',
                            {'job_id': tree['upstream']['job_id']}))
                        assert tree_result['status'] == 'exited'
                        assert 'DIRECT_GRANTED_BYTES_READ' in tree_output['upstream']['text']
                        assert tree_result['exit_code'] == 0, tree_result
                        assert 'MODULE_TREE_PASS:LIST_READ_WRITE_DENIED' in tree_output['upstream']['text']
                        assert not (project / 'src/forbidden-new.txt').exists()
                        print('NODE_MODULE_TREE_MODE: resolved; directory listing, sibling data and writes denied')
                        state.control('revoke', 'A')
                        assert (await local_stop(files, upstream))['session_revoked']
                        assert str(project) not in '\n'.join(logs)
                        assert 'function add' not in '\n'.join(logs)
                finally:
                    files.stopped = execution.stopped = True
                    server.should_exit = True
                    await asyncio.wait_for(service, 10)
                    listener.close()
    print('NODE HTTP PASS: isolated failing assertion -> exact-version source fix -> syntax and test success; unflagged entry denied, locally selected symlink-resolution flags permit file entry while ungranted sibling remains denied. No dependency/build/server proof.')


if __name__ == '__main__':
    logging.disable(logging.CRITICAL)
    asyncio.run(asyncio.wait_for(run(sys.argv[1], sys.argv[2]), timeout=90))
