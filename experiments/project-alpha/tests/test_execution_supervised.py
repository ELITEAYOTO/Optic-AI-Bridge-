import asyncio
import unittest

from mcp.shared.exceptions import MCPError
from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from execution_gate import ExecutionGate
from test_execution import profiles


class Supervised(unittest.IsolatedAsyncioTestCase):
    async def test_job_slots_rate_window_and_explicit_terminal_release(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        now, serial, status = [0.0], [0], ['running']
        calls = []
        async def upstream(name, args):
            calls.append(name)
            if name == 'process_start':
                serial[0] += 1
                return {'job_id': f'{serial[0]:064x}'}
            return {'status': status[0], 'exit_code': 0, 'output_truncated': False}
        gate = ExecutionGate(state, owner, upstream, profiles(), supervised=True, clock=lambda: now[0])
        jobs = [(await gate.call(owner, 'optic_task_start', {'profile': 'check'}))['job_id'] for _ in range(16)]
        now[0] = 60
        with self.assertRaisesRegex(ForwardError, 'execution.job_capacity'):
            await gate.call(owner, 'optic_task_start', {'profile': 'check'})
        for pending in ('running', 'termination_uncertain'):
            status[0] = pending
            with self.assertRaisesRegex(ForwardError, 'execution.job_not_terminal'):
                await gate.call(owner, 'optic_task_forget', {'job_id': jobs[0]})
            self.assertEqual(len(gate.jobs), 16)
        status[0] = 'exited'
        for job in jobs:
            self.assertTrue((await gate.call(owner, 'optic_task_forget', {'job_id': job}))['forgotten'])
        for _ in range(16):
            await gate.call(owner, 'optic_task_start', {'profile': 'check'})
        self.assertEqual(gate.starts, 32)
        self.assertEqual(len(gate.jobs), 16)
        for job in tuple(gate.jobs):
            await gate.call(owner, 'optic_task_forget', {'job_id': job})
        before = len(calls)
        with self.assertRaisesRegex(ForwardError, 'execution.start_budget'):
            await gate.call(owner, 'optic_task_start', {'profile': 'check'})
        self.assertEqual(len(calls), before)
        now[0] = 120
        await gate.call(owner, 'optic_task_start', {'profile': 'check'})

    async def test_unknown_or_rotated_job_cannot_be_released(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        async def upstream(name, args):
            if name == 'process_start':
                return {'job_id': 'a' * 64}
            raise MCPError(-32602, 'optic.process_job_not_found')
        gate = ExecutionGate(state, owner, upstream, profiles(), supervised=True)
        with self.assertRaisesRegex(ForwardError, 'execution.invalid_arguments'):
            await gate.call(owner, 'optic_task_forget', {'job_id': 'b' * 64})
        job = (await gate.call(owner, 'optic_task_start', {'profile': 'check'}))['job_id']
        self.assertTrue((await gate.call(owner, 'optic_task_forget', {'job_id': job}))['forgotten'])
        job = (await gate.call(owner, 'optic_task_start', {'profile': 'check'}))['job_id']
        state.control('rotate', 'A')
        new_owner = state.authenticate(b'Bearer ' + TOKENS['A'][1])
        with self.assertRaisesRegex(ForwardError, 'execution.invalid_arguments'):
            await gate.call(new_owner, 'optic_task_forget', {'job_id': job})


    async def test_lost_reply_recovery_lists_only_owned_job_without_restarting(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        entered, release = asyncio.Event(), asyncio.Event()
        calls = []
        async def upstream(name, args):
            calls.append(name)
            entered.set()
            await release.wait()
            return {'job_id': 'c' * 64}
        gate = ExecutionGate(state, owner, upstream, profiles(), supervised=True)
        caller = asyncio.create_task(gate.call(owner, 'optic_task_start', {'profile': 'check'}))
        await entered.wait()
        caller.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await caller
        with self.assertRaisesRegex(ForwardError, 'link.busy'):
            await gate.call(owner, 'optic_task_list', {})
        release.set()
        async with asyncio.timeout(1):
            while gate.pending:
                await asyncio.sleep(0)
        listed = await gate.call(owner, 'optic_task_list', {})
        self.assertEqual(listed['jobs'], [{'job_id': 'c' * 64}])
        self.assertEqual(calls, ['process_start'])
        other = state.authenticate(b'Bearer ' + TOKENS['B'][0])
        with self.assertRaisesRegex(ForwardError, 'link.inactive'):
            await gate.call(other, 'optic_task_list', {})
        state.control('rotate', 'A')
        current = state.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertEqual((await gate.call(current, 'optic_task_list', {}))['jobs'], [])
