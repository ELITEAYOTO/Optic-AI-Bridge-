import asyncio
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'francestudent-probe'))
from server import Guard
from rate_core import Bucket, RateGate


class Clock:
    value = 0.0
    def __call__(self):
        return self.value


class BucketTests(unittest.TestCase):
    def test_exact_refill_and_capacity(self):
        clock = Clock()
        bucket = Bucket(clock)
        self.assertEqual([bucket.take() for _ in range(9)], [True] * 8 + [False])
        clock.value = .49
        self.assertFalse(bucket.take())
        clock.value = .5
        self.assertTrue(bucket.take())
        self.assertFalse(bucket.take())
        clock.value = 10000
        self.assertEqual(sum(bucket.take() for _ in range(100)), 8)

    def test_bad_clock_and_regression_cannot_mint(self):
        clock = Clock()
        bucket = Bucket(clock, capacity=1)
        self.assertTrue(bucket.take())
        clock.value = -20
        self.assertFalse(bucket.take())
        clock.value = float('nan')
        self.assertFalse(bucket.take())
        clock.value = 0
        self.assertFalse(bucket.take())
        clock.value = .5
        self.assertTrue(bucket.take())

    def test_invalid_limits(self):
        for capacity, rate in [(True, 2), (0, 2), (33, 2), (8, 0),
                               (8, float('inf')), (8, True), (8, 1e-320)]:
            with self.assertRaises(ValueError):
                Bucket(capacity=capacity, per_second=rate)


class GateTests(unittest.IsolatedAsyncioTestCase):
    async def request(self, app, headers=(), method='POST'):
        messages = []
        async def receive():
            raise AssertionError('rejected request must not read body')
        async def send(message):
            messages.append(message)
        await app({'type': 'http', 'method': method, 'path': '/mcp',
                   'query_string': b'', 'headers': list(headers)}, receive, send)
        return messages

    async def test_guard_auth_is_not_bypassed_and_rejection_is_redacted(self):
        clock = Clock()
        calls, logs = [], []
        async def downstream(scope, receive, send):
            calls.append(1)
        app = RateGate(Guard(downstream, clock=clock, emit=logs.append),
                       clock=clock, capacity=2)
        for _ in range(2):
            self.assertEqual((await self.request(app))[0]['status'], 401)
        log_count = len(logs)
        denied = await self.request(app, [(b'authorization', b'canary-secret'),
                                          (b'x-forwarded-for', b'arbitrary-peer')])
        self.assertEqual(denied[0]['status'], 429)
        self.assertEqual(json.loads(denied[1]['body']), {'code': 'probe.rate_limited'})
        self.assertNotIn('canary-secret', str(denied) + str(logs))
        self.assertEqual(len(logs), log_count)
        self.assertEqual(calls, [])
        clock.value = .5
        self.assertEqual((await self.request(app))[0]['status'], 401)

    async def test_concurrent_burst_and_input_churn_share_one_budget(self):
        entered = 0
        async def downstream(scope, receive, send):
            nonlocal entered
            entered += 1
            await asyncio.sleep(0)
            await send({'type': 'http.response.start', 'status': 204, 'headers': []})
        app = RateGate(downstream, clock=Clock())
        results = await asyncio.gather(*(self.request(app,
            [(b'authorization', str(i).encode()), (b'x-forwarded-for', str(i).encode())],
            method=('GET' if i % 2 else 'POST')) for i in range(100)))
        self.assertEqual(entered, 8)
        self.assertEqual(sum(r[0]['status'] == 429 for r in results), 92)
        self.assertEqual(set(vars(app.bucket)), {'clock', 'capacity', 'rate', 'last', 'tokens'})

    async def test_lifespan_does_not_spend_budget(self):
        seen = []
        async def downstream(scope, receive, send):
            seen.append(scope['type'])
        app = RateGate(downstream, clock=Clock(), capacity=1)
        await app({'type': 'lifespan'}, None, None)
        self.assertEqual(seen, ['lifespan'])
        self.assertTrue(app.bucket.take())


if __name__ == '__main__':
    unittest.main()
