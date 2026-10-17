import asyncio
import hashlib
import json
import logging
from pathlib import Path
import socket
import tempfile
import unittest

import httpx2
import uvicorn
from diagnostic_client import client, endpoint, measure, sample, save_report
from diagnostic_host import make_diagnostic_app
from http_smoke import headers
from lifecycle_core import TOKENS


class Diagnostic(unittest.IsolatedAsyncioTestCase):
    async def test_actual_http_receipt_proof_bounded_payload_delay_and_input_denial(self):
        lines = []
        app = make_diagnostic_app(lines.append)
        listener = socket.socket()
        listener.bind(('127.0.0.1', 0))
        listener.listen(8)
        url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
        previous = logging.root.manager.disable
        logging.disable(logging.CRITICAL)
        service = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False))
        task = asyncio.create_task(service.serve(sockets=[listener]))
        try:
            async with asyncio.timeout(5):
                while not service.started:
                    if task.done():
                        await task
                    await asyncio.sleep(.01)
            report = await measure(url, quick=True)
            self.assertIsNone(report['failure'])
            self.assertEqual([row['verified_bytes'] for row in report['records']], [128, 1024])
            for row in report['records']:
                self.assertTrue(any(row['marker'] in line for line in lines))
            async with client(url, TOKENS['A'][0]) as session:
                ids = set()
                row = await sample(session, 8192, 10, 0, ids)
                self.assertTrue(row['ok'], row)
                self.assertGreaterEqual(row['client_elapsed_ms'], 9900)
                self.assertIn(200, row['http_statuses'])
                self.assertTrue(all(status in (200, 202) for status in row['http_statuses']))
                for size in range(4608, 8192, 512):
                    row = await sample(session, size, 0, 0, ids)
                    self.assertTrue(row['ok'], row)
                    self.assertEqual(row['verified_bytes'], size)
            async with httpx2.AsyncClient(headers=headers(TOKENS['A'][0]), timeout=3) as http:
                for arguments in ({'size_bytes': True}, {'size_bytes': 4500},
                                  {'size_bytes': 8193}, {'delay_seconds': 11}, {'path': 'src/SECRET'}):
                    response = await http.post(url, json={'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call',
                        'params': {'name': 'diagnostic_sample', 'arguments': arguments}})
                    self.assertEqual(response.status_code, 400)
            self.assertNotIn('SECRET', '\n'.join(lines))
            with tempfile.TemporaryDirectory() as directory:
                target = Path(directory) / 'report.json'
                save_report(target, report)
                self.assertEqual(json.loads(target.read_bytes()), report)
                self.assertNotIn('authorization', target.read_text().lower())
                self.assertNotIn(url, target.read_text())
                with self.assertRaises(FileExistsError):
                    save_report(target, report)
        finally:
            service.should_exit = True
            await asyncio.wait_for(task, 5)
            listener.close()
            logging.disable(previous)

    async def test_bad_reply_and_exception_are_not_success_or_raw_diagnostics(self):
        class Broken:
            async def call_tool(self, *_):
                raise RuntimeError('SECRET-CANARY')
        result = await sample(Broken(), 128, 0, 0, set())
        self.assertFalse(result['ok'])
        self.assertNotIn('SECRET', json.dumps(result))
        class Incomplete:
            async def call_tool(self, *_):
                from types import SimpleNamespace
                return SimpleNamespace(is_error=False, structured_content={'ok': True, 'text': 'd'})
        result = await sample(Incomplete(), 128, 0, 0, set())
        self.assertFalse(result['ok'])

    def test_endpoint_rejects_credential_urls_and_cleartext_external_hosts(self):
        for value in ('https://user:secret@example.org/mcp', 'https://example.org/mcp?secret=x',
                      'http://example.org/mcp', 'https://example.org/other', 'file:///mcp'):
            with self.assertRaises(ValueError):
                endpoint(value)
        self.assertEqual(endpoint('https://public.example/mcp'), 'https://public.example/mcp')
