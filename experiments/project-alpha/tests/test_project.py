import asyncio
import base64
import json
import unittest

from mcp.shared.exceptions import MCPError
from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from project_gate import ProjectGate
from project_schema import normalize, SchemaError, MAX_CONTENT
from project_http import ProjectGuard


class Schema(unittest.TestCase):
    def test_paths_numeric_types_versions_and_unknown_fields(self):
        bad = [('../outside', {}), ('src/../x', {}), ('src/con.txt', {}), ('src/COM1.java', {}),
               ('src/x.', {}), ('src-other/x', {}), ('src/x:y', {}), ('src\\x', {}),
               ('src/a', {'max_bytes': True}), ('src/a', {'offset': -1}),
               ('src/a', {'max_bytes': 8193}), ('src/a', {'session': 'CANARY'})]
        for target, extra in bad:
            with self.assertRaises(SchemaError):
                normalize('optic_fs_read', {'path': target, **extra})
        for args in ({'path': 'src/x', 'content': '', 'expected_kind': 'content'},
                     {'path': 'src/x', 'content': '', 'expected_kind': 'content', 'expected_version': '0'},
                     {'path': 'src/x', 'content': '\ud800', 'expected_kind': 'absent'},
                     {'path': 'src/x', 'content': 'é' * MAX_CONTENT, 'expected_kind': 'absent'}):
            with self.assertRaises(SchemaError):
                normalize('optic_fs_write', args)
        name, request = normalize('optic_fs_write', {'path': 'src/Main.java', 'content': 'class Main {}',
            'expected_kind': 'absent', 'expected_version': None})
        self.assertEqual(name, 'fs_write')
        self.assertEqual(request['expected'], {'kind': 'absent'})

    def test_strict_json_duplicate_nonfinite_and_fixed_tool_allowlist(self):
        guard = ProjectGuard(None)
        for body in (b'{"method":"ping","method":"tools/call"}', b'{"x":NaN}',
                     b'{"params":{"arguments":{"path":"src/x","path":"src/y"}}}'):
            with self.assertRaises(ValueError):
                guard.decode_request(body)
        self.assertFalse(guard.valid_arguments('process_start', {}))
        self.assertFalse(guard.valid_arguments('optic_project_info', {'project': 'CANARY'}))


class Gate(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.state = Registry()
        self.identity = self.state.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.entered, self.release = asyncio.Event(), asyncio.Event()
        self.calls = []

    async def held(self, name, args):
        self.calls.append((name, args))
        self.entered.set()
        await self.release.wait()
        return {'content_version': '1' * 64}

    def write(self):
        return {'path': 'src/x', 'content': 'public', 'expected_kind': 'absent'}

    async def test_cancelled_write_retains_slot_until_actual_completion(self):
        gate = ProjectGate(self.state, self.identity, self.held)
        caller = asyncio.create_task(gate.call(self.identity, 'optic_fs_write', self.write()))
        await self.entered.wait()
        caller.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await caller
        self.assertEqual(len(gate.pending), 1)
        with self.assertRaisesRegex(ForwardError, 'link.busy'):
            await gate.call(self.identity, 'optic_fs_read', {'path': 'src/x'})
        self.assertFalse(await gate.stop(0))
        self.release.set()
        self.assertTrue(await gate.stop())
        self.assertEqual(len(self.calls), 1)

    async def test_timeout_and_revoke_suppress_reply_without_claiming_rollback(self):
        gate = ProjectGate(self.state, self.identity, self.held, deadline=.01)
        with self.assertRaisesRegex(ForwardError, 'project.outcome_unknown'):
            await gate.call(self.identity, 'optic_fs_write', self.write())
        self.assertEqual(len(gate.pending), 1)
        self.state.control('revoke', 'A')
        self.release.set()
        self.assertTrue(await gate.stop())
        self.assertEqual(len(self.calls), 1)  # already dispatched; no rollback claim

    async def test_revoke_before_dispatch_unknown_tool_and_other_owner_never_forward(self):
        gate = ProjectGate(self.state, self.identity, self.held)
        other = self.state.authenticate(b'Bearer ' + TOKENS['B'][0])
        for identity, tool, args in ((other, 'optic_fs_write', self.write()),
                                     (self.identity, 'session_cancel', {})):
            with self.assertRaises(ForwardError):
                await gate.call(identity, tool, args)
        caller = asyncio.create_task(gate.call(self.identity, 'optic_fs_write', self.write()))
        await asyncio.sleep(0)
        self.state.control('revoke', 'A')
        with self.assertRaises(ForwardError):
            await caller
        self.assertEqual(self.calls, [])

    async def test_known_errors_pass_unknown_exception_is_redacted(self):
        async def known(name, args):
            raise MCPError(-32000, 'optic.precondition_failed')
        gate = ProjectGate(self.state, self.identity, known)
        with self.assertRaisesRegex(ForwardError, '^optic.precondition_failed$'):
            await gate.call(self.identity, 'optic_fs_write', self.write())
        async def secret(name, args):
            raise RuntimeError('SECRET_CANARY_PATH')
        gate = ProjectGate(self.state, self.identity, secret)
        with self.assertRaisesRegex(ForwardError, '^project.outcome_unknown$'):
            await gate.call(self.identity, 'optic_fs_write', self.write())

    async def test_literal_search_and_binary_read_are_bounded(self):
        async def read(name, args):
            return {'path': 'src/x', 'data': base64.b64encode(b'a\na\na\n').decode(), 'eof': True}
        gate = ProjectGate(self.state, self.identity, read)
        result = await gate.call(self.identity, 'optic_fs_search', {'path': 'src/x', 'text': 'a', 'limit': 2})
        self.assertEqual([hit['line'] for hit in result['matches']], [1, 2])
        async def binary(name, args):
            return {'path': 'src/x', 'data': '/w==', 'eof': True}
        gate = ProjectGate(self.state, self.identity, binary)
        self.assertIsNone((await gate.call(self.identity, 'optic_fs_read', {'path': 'src/x'}))['text'])

    async def test_patch_growth_bound_and_revoke_after_preparatory_read(self):
        calls = []
        async def full(name, args):
            calls.append(name)
            return {'data': base64.b64encode(b'x' * MAX_CONTENT).decode(), 'eof': True}
        arguments = {'path': 'src/x', 'expected_version': 'a' * 64,
                     'offset': 0, 'remove_bytes': 0, 'insert': 'x'}
        gate = ProjectGate(self.state, self.identity, full)
        with self.assertRaisesRegex(ForwardError, 'project.patch_limit'):
            await gate.call(self.identity, 'optic_fs_patch', arguments)
        self.assertEqual(calls, ['fs_read'])
        calls.clear()
        async def revoked(name, args):
            calls.append(name)
            self.state.control('revoke', 'A')
            return {'data': 'eA==', 'eof': True}
        gate = ProjectGate(self.state, self.identity, revoked)
        with self.assertRaisesRegex(ForwardError, 'link.inactive'):
            await gate.call(self.identity, 'optic_fs_patch', arguments)
        self.assertEqual(calls, ['fs_read'])
