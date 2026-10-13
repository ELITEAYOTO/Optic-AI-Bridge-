import asyncio
import json
from pathlib import Path
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'francestudent-probe'))
from proof_channel import ProofChannel
from server import Guard, result
from smoke import header_map


class ChannelTests(unittest.TestCase):
    def test_blocked_sink_bounded_concurrent_producers_and_shutdown(self):
        entered, release = threading.Event(), threading.Event()
        def sink(line):
            entered.set()
            assert release.wait(2)
        channel = ProofChannel(sink, capacity=3)
        try:
            self.assertTrue(channel.emit('initial'))
            self.assertTrue(entered.wait(1))
            with ThreadPoolExecutor(max_workers=8) as pool:
                accepted = list(pool.map(lambda _: channel.emit('fixed safe line'), range(50)))
            self.assertEqual(sum(accepted), 3)
            snapshot = channel.snapshot()
            self.assertEqual(snapshot['queued'], 3)
            self.assertEqual(snapshot['accepted'], 4)
            self.assertEqual(snapshot['dropped'], 47)
            self.assertFalse(channel.close())
            self.assertFalse(channel.emit('after close'))
        finally:
            release.set()
            channel.worker.join(timeout=1)
            self.assertFalse(channel.worker.is_alive())

    def test_budget_input_bounds_and_saturating_counters(self):
        lines = []
        channel = ProofChannel(lines.append, budget=1)
        self.assertFalse(channel.emit('x' * 4097))
        self.assertFalse(channel.emit('\u00e9'))
        self.assertFalse(channel.emit({}))
        self.assertTrue(channel.emit('only accepted record'))
        for _ in range(10000):
            self.assertFalse(channel.emit('excess'))
        self.assertTrue(channel.close())
        self.assertEqual(lines, ['only accepted record'])
        self.assertEqual(channel.snapshot()['dropped'], 4096)

    def test_sink_exception_is_never_echoed_and_closes_producers(self):
        def sink(line):
            raise RuntimeError('SECRET-EXCEPTION-CANARY')
        channel = ProofChannel(sink)
        channel.emit('safe')
        channel.worker.join(timeout=1)
        self.assertFalse(channel.worker.is_alive())
        self.assertTrue(channel.snapshot()['failed'])
        self.assertFalse(channel.emit('late'))
        self.assertNotIn('SECRET-EXCEPTION-CANARY', repr(channel.snapshot()))
        self.assertTrue(channel.close())

    def test_invalid_fixed_configuration(self):
        for options in ({'capacity': True}, {'capacity': 129}, {'budget': 0}, {'sink': None}):
            with self.assertRaises(ValueError):
                ProofChannel(**options)


class TransportTests(unittest.IsolatedAsyncioTestCase):
    async def test_stalled_sink_does_not_prevent_cooperative_timeout_response(self):
        entered, release = threading.Event(), threading.Event()
        def sink(line):
            entered.set()
            assert release.wait(2)
        channel = ProofChannel(sink)
        channel.emit('warmup')
        self.assertTrue(await asyncio.to_thread(entered.wait, 1))
        async def stalled(scope, receive, send):
            await asyncio.sleep(10)
        guard = Guard(stalled, emit=channel.emit, request_timeout=.02, timed_diagnostics=True)
        headers = [(k.lower().encode(), v.encode()) for k, v in header_map().items()]
        messages = []
        async def receive():
            return {'type': 'http.request', 'body': b''}
        async def send(message):
            messages.append(message)
        try:
            await asyncio.wait_for(guard({'type': 'http', 'method': 'POST', 'path': '/mcp',
                'query_string': b'', 'headers': headers}, receive, send), timeout=.5)
            self.assertTrue(entered.is_set())
            self.assertEqual(messages[0]['status'], 408)
            self.assertEqual(guard.active, 0)
            self.assertEqual(channel.snapshot()['delivered'], 0)
            self.assertFalse(channel.snapshot()['failed'])
        finally:
            release.set()
            channel.close()

    async def test_tool_start_and_send_completion_timing_with_matching_marker(self):
        now = [0.0]
        lines, messages = [], []
        channel = ProofChannel(lines.append)
        async def upstream(scope, receive, send):
            now[0] = .123
            payload = result('synthetic_probe', {'ok': True, 'authority': 'none'})
            await send({'type': 'http.response.start', 'status': 200, 'headers': []})
            now[0] = .2
            await send({'type': 'http.response.body', 'body': json.dumps(payload).encode()})
        guard = Guard(upstream, clock=lambda: now[0], emit=channel.emit, timed_diagnostics=True)
        headers = [(k.lower().encode(), v.encode()) for k, v in header_map().items()]
        async def receive():
            return {'type': 'http.request', 'body': b''}
        async def send(message):
            messages.append(message)
        try:
            await guard({'type': 'http', 'method': 'POST', 'path': '/mcp',
                'query_string': b'', 'headers': headers}, receive, send)
        finally:
            self.assertTrue(channel.close())
        proof = json.loads(messages[-1]['body'])['probe_invocation']
        self.assertEqual(proof['request_elapsed_ms'], 123)
        self.assertTrue(proof['console_queued'])
        status = json.loads(next(line[len('[FS-PROBE] STATUS '):] for line in lines if line.startswith('[FS-PROBE] STATUS ')))
        complete = json.loads(next(line[len('[FS-PROBE] COMPLETE '):] for line in lines if line.startswith('[FS-PROBE] COMPLETE ')))
        self.assertEqual(status['request_elapsed_ms'], 123)
        self.assertEqual(complete['request_elapsed_ms'], 200)
        self.assertEqual(complete['proof_version'], 'fs-probe-v3')
        self.assertTrue(any(proof['marker'] in line for line in lines))
