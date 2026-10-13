import asyncio
import logging
import socket
import unittest

import uvicorn
from forward_core import ForwardGate
from lifecycle_core import Registry, TOKENS
from http_adapter import make_app
from http_smoke import client, denied, structured


class Tests(unittest.IsolatedAsyncioTestCase):
    async def test_loopback_sdk_surface_fixed_arguments_and_reconnect(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        calls = []
        async def upstream(name, arguments):
            calls.append((name, arguments))
            return {'fixed': True}
        gate = ForwardGate(state, identity, upstream)
        logs = []
        app = make_app(gate, state, emit=logs.append)
        listener = socket.socket()
        listener.bind(('127.0.0.1', 0))
        listener.listen(32)
        url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
        previous = logging.root.manager.disable
        logging.disable(logging.CRITICAL)
        server = uvicorn.Server(uvicorn.Config(app, log_config=None,
            access_log=False, timeout_graceful_shutdown=2))
        task = asyncio.create_task(server.serve(sockets=[listener]))
        try:
            async with asyncio.timeout(5):
                while not server.started:
                    if task.done():
                        await task
                        self.fail('startup failed')
                    await asyncio.sleep(.01)
            async with client(url, TOKENS['A'][0]) as session:
                names = {item.name for item in (await session.list_tools()).tools}
                self.assertEqual(names, {'optic_session_info', 'optic_fixture_list', 'optic_fixture_read'})
                payload = structured(await session.call_tool('optic_fixture_read', {}))
                self.assertTrue(payload['ok'])
            self.assertEqual(calls, [('fs_read', {'path': 'public/hello.txt', 'max_bytes': 1024})])
            await denied(url, TOKENS['B'][0])
            await denied(url, TOKENS['A'][0], 400, {'jsonrpc': '2.0', 'id': 1,
                'method': 'tools/call', 'params': {'name': 'optic_fixture_read',
                'arguments': {'path': 'CALLER_CANARY'}}})
            state.control('rotate', 'A')
            await denied(url, TOKENS['A'][0])
            async with client(url, TOKENS['A'][1]) as session:
                self.assertTrue(structured(await session.call_tool('optic_session_info', {}))['ok'])
            await gate.stop()
            await denied(url, TOKENS['A'][1])
            self.assertEqual(len(calls), 2)
            self.assertNotIn('CALLER_CANARY', '\n'.join(logs))
        finally:
            server.should_exit = True
            await asyncio.wait_for(task, 5)
            listener.close()
            logging.disable(previous)
