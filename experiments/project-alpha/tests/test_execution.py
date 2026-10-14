import asyncio
import copy
import unittest

from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from execution_gate import ExecutionGate, freeze_profiles


def profiles():
    return {'check': {'executable': 'C:/Selected/node.exe', 'args': ['--check', 'src/app.js'],
        'cwd': None, 'env_allowlist': [], 'network': False, 'timeout_ms': 10000,
        'output_budget': 65536, 'memory_bytes': 268435456, 'process_count': 1}}


class Execution(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.state = Registry()
        self.identity = self.state.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.calls = []
        self.serial = 0

    async def upstream(self, name, args):
        self.calls.append((name, args))
        if name == 'process_start':
            self.serial += 1
            return {'job_id': f'{self.serial:064x}'}
        if name == 'process_read':
            return {'data': 'b2s=', 'offset': 0, 'next_offset': 2, 'eof': True, 'truncated': False}
        if name == 'process_stop':
            return {'requested': True}
        return {'status': 'exited', 'exit_code': 0, 'output_truncated': False}

    async def test_fixed_profile_snapshot_and_owned_job_lifecycle(self):
        config = profiles()
        gate = ExecutionGate(self.state, self.identity, self.upstream, config)
        config['check']['args'][:] = ['-e', 'SECRET']
        job = (await gate.call(self.identity, 'optic_task_start', {'profile': 'check'}))['job_id']
        self.assertEqual(self.calls[0], ('process_start', profiles()['check']))
        self.assertEqual((await gate.call(self.identity, 'optic_task_result', {'job_id': job}))['status'], 'exited')
        output = await gate.call(self.identity, 'optic_task_read',
            {'job_id': job, 'stream': 'stdout', 'cursor': 0, 'max_bytes': 1024})
        self.assertEqual(output['text'], 'ok')
        self.assertNotIn('data', output)
        self.assertTrue((await gate.call(self.identity, 'optic_task_stop', {'job_id': job}))['requested'])
        self.assertTrue(await gate.stop())

    async def test_caller_cannot_change_permissions_arguments_or_select_arbitrary_job(self):
        gate = ExecutionGate(self.state, self.identity, self.upstream, profiles())
        for tool, args in [('optic_task_start', {'profile': 'check', 'network': True}),
                           ('optic_task_start', {'profile': 'unknown'}),
                           ('optic_task_start', {'profile': 'check', 'args': ['SECRET']}),
                           ('optic_task_stop', {'pid': 123}),
                           ('optic_task_stop', {'job_id': 'a' * 64})]:
            with self.assertRaises(ForwardError):
                await gate.call(self.identity, tool, args)
        other = self.state.authenticate(b'Bearer ' + TOKENS['B'][0])
        with self.assertRaisesRegex(ForwardError, 'link.inactive'):
            await gate.call(other, 'optic_task_start', {'profile': 'check'})
        self.assertFalse(self.calls)

    async def test_rotation_invalidates_old_job_access_and_start_budget_is_finite(self):
        gate = ExecutionGate(self.state, self.identity, self.upstream, profiles())
        job = (await gate.call(self.identity, 'optic_task_start', {'profile': 'check'}))['job_id']
        self.state.control('rotate', 'A')
        current = self.state.authenticate(b'Bearer ' + TOKENS['A'][1])
        with self.assertRaises(ForwardError):
            await gate.call(current, 'optic_task_result', {'job_id': job})
        for _ in range(15):
            await gate.call(current, 'optic_task_start', {'profile': 'check'})
        with self.assertRaisesRegex(ForwardError, 'execution.start_budget'):
            await gate.call(current, 'optic_task_start', {'profile': 'check'})
        self.assertEqual(len(gate.jobs), 16)

    async def test_timed_out_start_retains_slot_and_revoke_suppresses_success(self):
        entered, release = asyncio.Event(), asyncio.Event()
        async def held(name, args):
            entered.set()
            await release.wait()
            return {'job_id': 'a' * 64}
        gate = ExecutionGate(self.state, self.identity, held, profiles(), deadline=.01)
        with self.assertRaisesRegex(ForwardError, 'execution.outcome_unknown'):
            await gate.call(self.identity, 'optic_task_start', {'profile': 'check'})
        self.assertTrue(entered.is_set())
        with self.assertRaisesRegex(ForwardError, 'link.busy'):
            await gate.call(self.identity, 'optic_task_start', {'profile': 'check'})
        self.state.control('revoke', 'A')
        release.set()
        self.assertTrue(await gate.stop())
        self.assertFalse(gate.jobs)
        self.assertEqual(gate.starts, 1)

    async def test_cancelled_start_retains_slot_and_unknown_error_does_not_echo(self):
        entered, release = asyncio.Event(), asyncio.Event()
        async def held(name, args):
            entered.set()
            await release.wait()
            raise RuntimeError('SECRET_PATH')
        gate = ExecutionGate(self.state, self.identity, held, profiles())
        caller = asyncio.create_task(gate.call(self.identity, 'optic_task_start', {'profile': 'check'}))
        await entered.wait()
        caller.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await caller
        self.assertFalse(await gate.stop(0))
        release.set()
        self.assertTrue(await gate.stop())
        self.assertFalse(gate.jobs)

    async def test_profile_and_output_bounds(self):
        for field, value in [('network', True), ('env_allowlist', ['SECRET']),
                             ('process_count', 2), ('timeout_ms', True), ('output_budget', 65537)]:
            config = copy.deepcopy(profiles())
            config['check'][field] = value
            with self.assertRaises(ValueError):
                freeze_profiles(config)
        gate = ExecutionGate(self.state, self.identity, self.upstream, profiles())
        job = (await gate.call(self.identity, 'optic_task_start', {'profile': 'check'}))['job_id']
        with self.assertRaisesRegex(ForwardError, 'execution.upstream_failed'):
            await gate.call(self.identity, 'optic_task_read',
                {'job_id': job, 'stream': 'stdout', 'cursor': 0, 'max_bytes': 1})
