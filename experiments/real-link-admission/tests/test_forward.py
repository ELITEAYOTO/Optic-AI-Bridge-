import asyncio
import unittest

from forward_core import ForwardGate, ForwardError
from lifecycle_core import Registry, TOKENS


class Tests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.state = Registry()
        self.a = self.state.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.b = self.state.authenticate(b'Bearer ' + TOKENS['B'][0])
        self.enter = asyncio.Event()
        self.release = asyncio.Event()
        self.calls = []

    async def upstream(self, name, arguments):
        self.calls.append((name, arguments))
        self.enter.set()
        await self.release.wait()
        return {'public': True}

    async def test_fixed_calls_and_other_owner_denial(self):
        gate = ForwardGate(self.state, self.a, self.upstream)
        for identity, tool in ((self.b, 'optic_fixture_read'), (self.a, 'fs_write')):
            with self.assertRaises(ForwardError):
                await gate.call(identity, tool)
        self.assertEqual(self.calls, [])
        self.release.set()
        self.assertEqual(await gate.call(self.a, 'optic_fixture_read'), {'public': True})
        self.assertEqual(self.calls, [('fs_read', {'path': 'public/hello.txt', 'max_bytes': 1024})])

    async def test_cancelled_caller_retains_capacity_until_upstream_finishes(self):
        gate = ForwardGate(self.state, self.a, self.upstream, capacity=1)
        caller = asyncio.create_task(gate.call(self.a, 'optic_fixture_read'))
        await self.enter.wait()
        caller.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await caller
        with self.assertRaisesRegex(ForwardError, 'link.busy'):
            await gate.call(self.a, 'optic_fixture_read')
        self.assertEqual(len(gate.pending), 1)
        self.release.set()
        self.assertTrue(await gate.stop())
        self.assertEqual(len(self.calls), 1)

    async def test_timeout_retains_capacity_and_stop_cannot_reopen(self):
        gate = ForwardGate(self.state, self.a, self.upstream, capacity=1, deadline=.01)
        with self.assertRaisesRegex(ForwardError, 'link.response_timeout'):
            await gate.call(self.a, 'optic_fixture_read')
        self.assertFalse(await gate.stop(0))
        self.assertEqual(len(gate.pending), 1)
        self.release.set()
        self.assertTrue(await gate.stop())
        with self.assertRaisesRegex(ForwardError, 'link.inactive'):
            await gate.call(self.a, 'optic_fixture_read')

    async def test_rotation_suppresses_old_result_new_credential_keeps_binding(self):
        gate = ForwardGate(self.state, self.a, self.upstream)
        caller = asyncio.create_task(gate.call(self.a, 'optic_fixture_read'))
        await self.enter.wait()
        self.state.control('rotate', 'A')
        self.release.set()
        with self.assertRaisesRegex(ForwardError, 'link.inactive'):
            await caller
        fresh = self.state.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertEqual(fresh.principal, self.a.principal)
        self.assertEqual(await gate.call(fresh, 'optic_session_info'), {'public': True})
        self.state.control('revoke', 'A')
        with self.assertRaises(ForwardError):
            await gate.call(fresh, 'optic_session_info')

    async def test_rotation_before_worker_dispatch_and_redacted_exception(self):
        gate = ForwardGate(self.state, self.a, self.upstream)
        caller = asyncio.create_task(gate.call(self.a, 'optic_fixture_read'))
        await asyncio.sleep(0)  # caller reserved; worker has not dispatched
        self.state.control('revoke', 'A')
        with self.assertRaises(ForwardError):
            await caller
        self.assertEqual(self.calls, [])
        async def failing(name, arguments):
            raise RuntimeError('SECRET PATH CANARY')
        gate = ForwardGate(self.state, self.b, failing)
        with self.assertRaises(ForwardError) as error:
            await gate.call(self.b, 'optic_fixture_list')
        self.assertEqual(str(error.exception), 'link.upstream_failed')

    async def test_invalid_bounds(self):
        for options in ({'capacity': True}, {'capacity': 9}, {'deadline': float('nan')}, {'deadline': 16}):
            with self.assertRaises(ValueError):
                ForwardGate(self.state, self.a, self.upstream, **options)

    async def test_concurrent_pressure_and_stop_suppress_all_late_results(self):
        gate = ForwardGate(self.state, self.a, self.upstream)
        callers = [asyncio.create_task(gate.call(self.a, 'optic_fixture_read')) for _ in range(12)]
        await self.enter.wait()
        self.assertEqual(len(gate.pending), 8)
        self.assertFalse(await gate.stop(0))
        self.release.set()
        results = await asyncio.gather(*callers, return_exceptions=True)
        self.assertEqual(len(self.calls), 8)
        self.assertTrue(all(isinstance(item, ForwardError) for item in results))
        self.assertTrue(await gate.stop())
        self.assertEqual(len(gate.pending), 0)
