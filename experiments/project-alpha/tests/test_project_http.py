import asyncio
import base64
import logging
import socket
import unittest

import httpx2
import uvicorn
from lifecycle_core import Registry, TOKENS
from http_smoke import client, headers, structured
from project_schema import TOOLS
from project_gate import ProjectGate
from project_http import make_project_app


class Http(unittest.IsolatedAsyncioTestCase):
    async def test_exact_surface_schemas_safe_logs_reconnect_and_revoke(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        calls = []
        async def upstream(name, args):
            calls.append((name, args))
            if name == 'fs_read':
                return {'path': args['path'], 'data': base64.b64encode(b'public line\n').decode(),
                        'encoding': 'base64', 'eof': True, 'content_version': 'a' * 64}
            return {'public': True}
        gate = ProjectGate(state, identity, upstream)
        logs = []
        app = make_project_app(gate, state, emit=logs.append)
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
                        self.fail('startup failed')
                    await asyncio.sleep(.01)
            async with client(url, TOKENS['A'][0]) as session:
                self.assertEqual({tool.name for tool in (await session.list_tools()).tools}, set(TOOLS))
                value = structured(await session.call_tool('optic_fs_read', {'path': 'src/Main.java'}))
                self.assertEqual(value['upstream']['text'], 'public line\n')
                written = structured(await session.call_tool('optic_fs_write', {
                    'path': 'src/Main.java', 'content': 'class Main {}', 'expected_kind': 'absent'}))
                self.assertTrue(written['ok'])
            self.assertEqual(calls[1][0], 'fs_write')
            self.assertEqual(calls[1][1]['expected'], {'kind': 'absent'})
            before = len(calls)
            async with httpx2.AsyncClient(headers=headers(TOKENS['A'][0]), timeout=5) as http:
                for arguments in ({'path': '../CANARY'}, {'path': 'src/CON'},
                                  {'path': 'src/a', 'max_bytes': True}, {'path': 'src/a', 'project': 'CANARY'}):
                    response = await http.post(url, json={'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                        'params': {'name': 'optic_fs_read', 'arguments': arguments}})
                    self.assertEqual(response.status_code, 400)
                duplicate = await http.post(url, content=b'{"method":"tools/call","method":"ping"}')
                self.assertEqual(duplicate.status_code, 400)
                unknown = await http.post(url, json={'jsonrpc': '2.0', 'id': 2, 'method': 'tools/call',
                    'params': {'name': 'process_start', 'arguments': {}}})
                self.assertEqual(unknown.status_code, 400)
            self.assertEqual(len(calls), before)
            state.control('rotate', 'A')
            async with httpx2.AsyncClient(headers=headers(TOKENS['A'][0]), timeout=5) as http:
                self.assertEqual((await http.post(url, json={'method': 'ping'})).status_code, 401)
            async with client(url, TOKENS['A'][1]) as session:
                self.assertTrue(structured(await session.call_tool('optic_project_info', {}))['ok'])
            state.control('revoke', 'A')
            async with httpx2.AsyncClient(headers=headers(TOKENS['A'][1]), timeout=5) as http:
                self.assertEqual((await http.post(url, json={'method': 'ping'})).status_code, 401)
            self.assertNotIn('CANARY', '\n'.join(logs))
            self.assertNotIn('class Main', '\n'.join(logs))
            self.assertTrue(await gate.stop())
        finally:
            service.should_exit = True
            await asyncio.wait_for(task, 5)
            listener.close()
            logging.disable(disabled)
