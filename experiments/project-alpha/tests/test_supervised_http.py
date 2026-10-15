import asyncio
import logging
import socket
import unittest

import uvicorn
from lifecycle_core import Registry, TOKENS
from http_smoke import client
from project_results import structured
from project_schema import TOOLS, DIRECTORY_TOOL
from project_gate import ProjectGate
from project_http import make_project_app
from execution_gate import ExecutionGate, TASK_TOOLS, FORGET_TOOL, SUPERVISED_TASK_TOOLS
from supervision_limits import ProjectFiles, MAX_SEQUENCE
from test_execution import profiles


class Http(unittest.IsolatedAsyncioTestCase):
    async def test_modes_limits_long_idle_new_transport_and_terminal_release(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        now, status = [0.0], ['running']
        async def upstream(name, args):
            if name == 'fs_create_directory':
                return {'path': args['path'], 'created': True}
            if name == 'process_start':
                return {'job_id': 'a' * 64}
            if name == 'process_result':
                return {'status': status[0], 'exit_code': 0, 'output_truncated': False}
            return {'capabilities': ['file_read', 'file_write', 'process_run']}
        files = ProjectGate(state, owner, upstream, file_budget=ProjectFiles(), directory_create=True)
        execution = ExecutionGate(state, owner, upstream, profiles(), supervised=True, clock=lambda: now[0])
        app = make_project_app(files, state, execution=execution, supervised=True, clock=lambda: now[0], emit=lambda _: None)
        listener = socket.socket()
        listener.bind(('127.0.0.1', 0))
        listener.listen(32)
        url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
        disabled = logging.root.manager.disable
        logging.disable(logging.CRITICAL)
        service = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False, timeout_graceful_shutdown=2))
        task = asyncio.create_task(service.serve(sockets=[listener]))
        try:
            async with asyncio.timeout(5):
                while not service.started:
                    if task.done():
                        await task
                    await asyncio.sleep(.01)
            async with client(url, TOKENS['A'][0]) as session:
                self.assertEqual({tool.name for tool in (await session.list_tools()).tools}, set(TOOLS + TASK_TOOLS + SUPERVISED_TASK_TOOLS + (DIRECTORY_TOOL,)))
                info = structured(await session.call_tool('optic_project_info', {}))
                self.assertEqual(info['connection']['mode'], 'supervised')
                self.assertIsNone(info['connection']['remaining_seconds'])
                self.assertEqual(info['limits']['generated_directories'], 128)
                made = structured(await session.call_tool(DIRECTORY_TOOL, {'path': 'src/modules', 'expected_kind': 'absent'}))
                self.assertEqual(made['upstream'], {'path': 'src/modules', 'created': True})
                job = structured(await session.call_tool('optic_task_start', {'profile': 'check'}))['upstream']['job_id']
                denied = structured(await session.call_tool(FORGET_TOOL, {'job_id': job}))
                self.assertEqual(denied['code'], 'execution.job_not_terminal')
                listed = structured(await session.call_tool('optic_task_list', {}))
                self.assertEqual(listed['upstream']['jobs'], [{'job_id': job}])
                status[0] = 'exited'
                released = structured(await session.call_tool(FORGET_TOOL, {'job_id': job}))
                self.assertTrue(released['upstream']['forgotten'])
            # Simulate a later fixed window, beyond the old lifetime and request cap.
            now[0] = 3601.0
            app.sequence = 512
            async with client(url, TOKENS['A'][0]) as session:
                info = structured(await session.call_tool('optic_project_info', {}))
                self.assertTrue(info['ok'])
                self.assertGreater(app.sequence, 512)
                self.assertEqual(info['limits']['generated_files'], 128)
                self.assertEqual(info['limits']['retained_jobs'], 16)
            self.assertTrue(await files.stop())
        finally:
            service.should_exit = True
            await asyncio.wait_for(task, 5)
            listener.close()
            logging.disable(disabled)

    def test_admission_windows_do_not_reset_on_reconnect_and_clock_failure_is_terminal(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        async def upstream(name, args):
            return {}
        gate = ProjectGate(state, owner, upstream, file_budget=ProjectFiles())
        app = make_project_app(gate, state, supervised=True, clock=lambda: 0.0)
        self.assertTrue(all(app.admit_request(0) for _ in range(512)))
        self.assertFalse(app.admit_request(59))
        self.assertTrue(app.admit_request(60))
        self.assertEqual(app.sequence, 513)
        self.assertFalse(app.admit_request(59))
        self.assertFalse(app.admit_request(120))
        other = make_project_app(gate, state, supervised=True, clock=lambda: 0.0)
        other.sequence = MAX_SEQUENCE
        self.assertFalse(other.admit_request(0))
