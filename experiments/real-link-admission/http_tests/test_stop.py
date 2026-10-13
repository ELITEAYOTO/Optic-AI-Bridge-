import asyncio
from types import SimpleNamespace
import unittest
from unittest.mock import AsyncMock

from lifecycle_core import Registry, TOKENS
from forward_core import ForwardGate, ForwardError
from http_adapter import local_stop


class Tests(unittest.IsolatedAsyncioTestCase):
    async def test_failed_actual_revoke_does_not_reopen_admission(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        gate = ForwardGate(state, identity, AsyncMock())
        upstream = SimpleNamespace(call_tool=AsyncMock(side_effect=RuntimeError('RAW_CANARY')))
        self.assertEqual(await local_stop(gate, upstream), {
            'ok': False, 'code': 'link.session_revoke_failed'})
        with self.assertRaises(ForwardError):
            await gate.call(identity, 'optic_fixture_read')
        upstream.call_tool.assert_awaited_once_with('session_cancel', {})

    async def test_sdk_tool_error_is_not_acknowledged_as_revoke(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        gate = ForwardGate(state, identity, AsyncMock())
        upstream = SimpleNamespace(call_tool=AsyncMock(return_value=SimpleNamespace(is_error=True)))
        self.assertFalse((await local_stop(gate, upstream))['ok'])
        self.assertTrue(gate.stopped)

    async def test_revoke_is_invoked_only_after_drained_gate(self):
        state = Registry()
        identity = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        release = asyncio.Event()
        entered = asyncio.Event()
        async def read(name, arguments):
            entered.set()
            await release.wait()
            return {'public': True}
        gate = ForwardGate(state, identity, read)
        caller = asyncio.create_task(gate.call(identity, 'optic_fixture_read'))
        await entered.wait()
        async def revoke(name, arguments):
            self.assertTrue(gate.stopped)
            self.assertEqual(len(gate.pending), 0)
            return SimpleNamespace(is_error=False)
        upstream = SimpleNamespace(call_tool=AsyncMock(side_effect=revoke))
        stop = asyncio.create_task(local_stop(gate, upstream))
        await asyncio.sleep(0)
        upstream.call_tool.assert_not_awaited()
        release.set()
        with self.assertRaises(ForwardError):
            await caller
        self.assertTrue((await stop)['session_revoked'])
