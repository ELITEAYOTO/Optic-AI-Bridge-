import asyncio
import base64
import hashlib
from pathlib import Path
import tempfile
import unittest

from mcp.shared.exceptions import MCPError
from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from project_gate import ProjectGate
from project_schema import SchemaError, normalize
from tool_checkpoint import ToolCheckpoint


class Replace(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.state = Registry()
        self.owner = self.state.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.raw = 'é🙂\r\nold\r\ntail'.encode()
        self.calls = []
        self.before_patch = None

    def digest(self):
        return hashlib.sha256(self.raw).hexdigest()

    def arguments(self, old='old', new='new €', **extra):
        return {'path': 'src/a.js', 'expected_version': self.digest(),
                'old_text': old, 'new_text': new, **extra}

    async def upstream(self, name, args):
        self.calls.append((name, args))
        if name == 'fs_read':
            return {'path': args['path'], 'offset': 0, 'eof': True,
                    'content_version': self.digest(), 'data': base64.b64encode(self.raw).decode()}
        self.assertEqual(name, 'fs_apply_patch')
        if self.before_patch:
            self.before_patch()
        if args['expected_version'] != self.digest():
            raise MCPError(-32000, 'optic.precondition_failed')
        first, last = args['offset'], args['offset'] + args['remove_bytes']
        self.raw = self.raw[:first] + base64.b64decode(args['insert_base64']) + self.raw[last:]
        return {'path': args['path'], 'content_version': self.digest()}

    async def test_utf8_crlf_and_empty_replacement_preserve_untouched_bytes(self):
        gate = ProjectGate(self.state, self.owner, self.upstream)
        result = await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        self.assertEqual(self.raw, 'é🙂\r\nnew €\r\ntail'.encode())
        self.assertEqual(result['content_version'], self.digest())
        self.assertEqual(self.calls[1][1]['offset'], len('é🙂\r\n'.encode()))
        self.assertEqual(set(self.calls[1][1]), {'path', 'expected_version', 'offset', 'remove_bytes', 'insert_base64'})
        await gate.call(self.owner, 'optic_fs_replace', self.arguments('new €', ''))
        self.assertEqual(self.raw, 'é🙂\r\n\r\ntail'.encode())

    async def test_missing_multiple_overlapping_and_stale_matches_never_patch(self):
        for raw, old, version, code in [
            (b'none', 'old', None, 'project.match_not_found'),
            (b'old old', 'old', None, 'project.match_ambiguous'),
            (b'aaa', 'aa', None, 'project.match_ambiguous'),
            (b'old', 'old', '0' * 64, 'optic.precondition_failed'),
            (b'\xffold', 'old', None, 'project.text_encoding'),
        ]:
            with self.subTest(code=code, raw=raw):
                self.raw, self.calls = raw, []
                gate = ProjectGate(self.state, self.owner, self.upstream)
                args = self.arguments(old, expected_version=version or self.digest())
                with self.assertRaisesRegex(ForwardError, '^' + code + '$'):
                    await gate.call(self.owner, 'optic_fs_replace', args)
                self.assertEqual(self.raw, raw)
                self.assertEqual([name for name, _ in self.calls], ['fs_read'])

    async def test_size_caps_and_changed_after_read_are_independently_checked(self):
        for cap in (8192, 32768):
            self.raw, self.calls = b'x' * (cap - 1) + b'!', []
            gate = ProjectGate(self.state, self.owner, self.upstream, max_file_bytes=cap)
            with self.assertRaisesRegex(ForwardError, '^project.patch_limit$'):
                await gate.call(self.owner, 'optic_fs_replace', self.arguments('!', '€'))
            self.assertEqual(len(self.calls), 1)
            await gate.call(self.owner, 'optic_fs_replace', self.arguments('!', 'Z'))
            self.assertEqual(self.raw[-1:], b'Z')
        self.raw, self.calls = b'old', []
        self.before_patch = lambda: setattr(self, 'raw', b'EXTERNAL EDIT')
        gate = ProjectGate(self.state, self.owner, self.upstream)
        with self.assertRaisesRegex(ForwardError, '^optic.precondition_failed$'):
            await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        self.assertEqual(self.raw, b'EXTERNAL EDIT')
        self.assertEqual([name for name, _ in self.calls], ['fs_read', 'fs_apply_patch'])

    async def test_wrong_or_partial_snapshot_cannot_reach_mutation(self):
        for extra in ({'path': 'src/other.js'}, {'offset': False}, {'offset': 1},
                      {'eof': False}, {'eof': 1}, {'data': base64.b64encode(b'x' * 8193).decode()}):
            calls = []
            async def bad(name, args):
                calls.append(name)
                return {'path': 'src/a.js', 'offset': 0, 'eof': True,
                        'data': base64.b64encode(b'old').decode(),
                        'content_version': self.digest(), **extra}
            gate = ProjectGate(self.state, self.owner, bad)
            with self.assertRaisesRegex(ForwardError, '^project.replace_limit$'):
                await gate.call(self.owner, 'optic_fs_replace', self.arguments())
            self.assertEqual(calls, ['fs_read'])

    async def test_denied_read_or_revoked_owner_cannot_reach_patch(self):
        async def denied(name, args):
            self.calls.append(name)
            raise MCPError(-32000, 'optic.policy_denied')
        gate = ProjectGate(self.state, self.owner, denied)
        with self.assertRaisesRegex(ForwardError, '^optic.policy_denied$'):
            await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        self.assertEqual(self.calls, ['fs_read'])
        async def revoked(name, args):
            value = await self.upstream(name, args)
            self.state.control('revoke', 'A')
            return value
        self.calls = []
        gate = ProjectGate(self.state, self.owner, revoked)
        with self.assertRaises(ForwardError):
            await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        self.assertEqual([name for name, _ in self.calls], ['fs_read'])

    async def test_cancelled_caller_keeps_one_slot_and_never_retries_mutation(self):
        entered, release = asyncio.Event(), asyncio.Event()
        async def held(name, args):
            if name == 'fs_read':
                entered.set()
                await release.wait()
            return await self.upstream(name, args)
        gate = ProjectGate(self.state, self.owner, held)
        caller = asyncio.create_task(gate.call(self.owner, 'optic_fs_replace', self.arguments()))
        await entered.wait()
        caller.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await caller
        with self.assertRaisesRegex(ForwardError, '^link.busy$'):
            await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        retained = next(iter(gate.pending))
        release.set()
        await asyncio.shield(retained)
        self.assertEqual([name for name, _ in self.calls], ['fs_read', 'fs_apply_patch'])
        self.assertTrue(await gate.stop())

    async def test_timeout_is_uncertain_and_retained_work_is_not_replayed(self):
        entered, release = asyncio.Event(), asyncio.Event()
        async def held(name, args):
            if name == 'fs_apply_patch':
                entered.set()
                await release.wait()
            return await self.upstream(name, args)
        gate = ProjectGate(self.state, self.owner, held, deadline=.01)
        with self.assertRaisesRegex(ForwardError, '^project.outcome_unknown$'):
            await gate.call(self.owner, 'optic_fs_replace', self.arguments())
        self.assertTrue(entered.is_set())
        retained = next(iter(gate.pending))
        release.set()
        await asyncio.shield(retained)
        self.assertEqual([name for name, _ in self.calls], ['fs_read', 'fs_apply_patch'])

    def test_schema_and_retained_mutation_observation(self):
        for extra in ({'old_text': ''}, {'old_text': '\ud800'}, {'new_text': '\ud800'},
                      {'expected_version': None}, {'old_text': 'é' * 8192}, {'replace_all': True}):
            with self.assertRaises(SchemaError):
                normalize('optic_fs_replace', self.arguments(**extra))
        with tempfile.TemporaryDirectory() as directory:
            checkpoint = ToolCheckpoint(Path(directory))
            checkpoint.begin('optic_fs_replace', 1)
            event = ToolCheckpoint(Path(directory)).previous['event']
            self.assertTrue(event['outcome_uncertain'])
            self.assertEqual(event['phase'], 'admitted')
