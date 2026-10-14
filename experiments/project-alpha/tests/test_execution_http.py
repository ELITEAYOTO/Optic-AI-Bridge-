"""Actual SDK HTTP composition; fake upstream, not a native Node proof."""
import asyncio
import logging
import socket
import unittest

import httpx2
import uvicorn
from http_smoke import client, headers
from lifecycle_core import Registry, TOKENS
from project_results import structured
from project_schema import TOOLS
from project_gate import ProjectGate
from project_http import make_project_app
from execution_gate import ExecutionGate, TASK_TOOLS
from test_execution import profiles


class ExecutionHttp(unittest.IsolatedAsyncioTestCase):
    async def test_explicit_ten_tool_surface_profile_run_output_and_no_caller_grants(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        calls = []
        async def upstream(name, args):
            calls.append((name, args))
            if name == 'process_start':
                return {'job_id': 'a' * 64}
            if name == 'process_read':
                return {'data': 'b2s=', 'offset': 0, 'next_offset': 2, 'eof': True, 'truncated': False}
            if name == 'process_stop':
                return {'requested': True}
            return {'status': 'exited', 'exit_code': 0, 'output_truncated': False}
        files = ProjectGate(state, identity, upstream)
        execution = ExecutionGate(state, identity, upstream, profiles())
        logs = []
        app = make_project_app(files, state, emit=logs.append, execution=execution)
        listener = socket.socket()
        listener.bind(('127.0.0.1', 0))
        listener.listen(32)
        url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
        disabled = logging.root.manager.disable
        logging.disable(logging.CRITICAL)
        service = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False,
                                               timeout_graceful_shutdown=2))
        task = asyncio.create_task(service.serve(sockets=[listener]))
        try:
            async with asyncio.timeout(5):
                while not service.started:
                    if task.done():
                        await task
                    await asyncio.sleep(.01)
            async with client(url, TOKENS['A'][0]) as session:
                self.assertEqual({tool.name for tool in (await session.list_tools()).tools}, set(TOOLS + TASK_TOOLS))
                value = structured(await session.call_tool('optic_task_start', {'profile': 'check'}))
                self.assertTrue(value['ok'])
                job = value['upstream']['job_id']
                self.assertEqual(calls[0], ('process_start', profiles()['check']))
                result = structured(await session.call_tool('optic_task_result', {'job_id': job}))
                self.assertEqual(result['upstream']['status'], 'exited')
                output = structured(await session.call_tool('optic_task_read', {'job_id': job}))
                self.assertEqual(output['upstream']['text'], 'ok')
                self.assertTrue(structured(await session.call_tool('optic_task_stop', {'job_id': job}))['ok'])
                unknown = structured(await session.call_tool('optic_task_result', {'job_id': 'b' * 64}))
                self.assertFalse(unknown['ok'])
            before = len(calls)
            async with httpx2.AsyncClient(headers=headers(TOKENS['A'][0]), timeout=5) as http:
                invalid = await http.post(url, json={'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                    'params': {'name': 'optic_task_start', 'arguments': {'profile': 'check', 'network': True}}})
                self.assertEqual(invalid.status_code, 400)
            self.assertEqual(before, len(calls))
            self.assertNotIn('a' * 64, '\n'.join(logs))
            self.assertNotIn('C:/Selected', '\n'.join(logs))
            self.assertTrue(await files.stop())
        finally:
            service.should_exit = True
            await asyncio.wait_for(task, 5)
            listener.close()
            logging.disable(disabled)
