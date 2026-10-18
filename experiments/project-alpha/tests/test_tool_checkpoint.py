import asyncio
from contextlib import asynccontextmanager
import json
import logging
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

import uvicorn
from lifecycle_core import Registry, TOKENS
from http_smoke import client
from project_results import structured
from project_gate import ProjectGate
from project_http import make_project_app
from supervision_limits import ProjectFiles
from tool_checkpoint import ToolCheckpoint, NAME, PENDING, MAX_BYTES, validate


class Records(unittest.TestCase):
    def test_reopen_retains_only_safe_fields_and_no_grants_or_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = ToolCheckpoint(root)
            self.assertEqual(log.previous['state'], 'not_available')
            ticket = log.begin('optic_fs_write', 7)
            self.assertIsNotNone(ticket)
            self.assertTrue(log.finish(ticket, {'ok': True, 'content': 'SECRET',
                'path': 'SECRET', 'application_handle': 'SECRET', 'job_id': 'SECRET'}))
            raw = (root / NAME).read_bytes()
            self.assertLessEqual(len(raw), MAX_BYTES)
            self.assertNotIn(b'SECRET', raw)
            recovered = ToolCheckpoint(root).previous
            self.assertEqual(recovered['event']['phase'], 'completed')
            self.assertTrue(recovered['event']['ok'])
            self.assertFalse(recovered['trusted_for_authorization'])
            self.assertEqual([p.name for p in root.iterdir()], [NAME])

    def test_abrupt_process_exit_keeps_admission_uncertain_not_executed(self):
        with tempfile.TemporaryDirectory() as directory:
            script = "from tool_checkpoint import ToolCheckpoint; import os,sys; assert ToolCheckpoint(sys.argv[1]).begin('optic_fs_patch',19)==19; os._exit(42)"
            result = subprocess.run([sys.executable, '-c', script, directory], timeout=10)
            self.assertEqual(result.returncode, 42)
            event = ToolCheckpoint(directory).previous['event']
            self.assertEqual(event['phase'], 'admitted')
            self.assertIsNone(event['ok'])
            self.assertTrue(event['outcome_uncertain'])
            self.assertNotIn('executed', event)

    def test_abrupt_exit_between_fsync_and_replace_recovers_the_newer_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            script = "from tool_checkpoint import ToolCheckpoint; import os,sys; log=ToolCheckpoint(sys.argv[1]); os.replace=lambda *args: os._exit(43); log.begin('optic_task_start',23)"
            result = subprocess.run([sys.executable, '-c', script, directory], timeout=10)
            self.assertEqual(result.returncode, 43)
            first = ToolCheckpoint(directory).previous
            self.assertTrue(first['interrupted_publication'])
            self.assertEqual(first['event']['sequence'], 23)
            second = ToolCheckpoint(directory).previous
            self.assertEqual(second['event'], first['event'])
            self.assertFalse(second['trusted_for_authorization'])

    def test_latest_admission_wins_over_older_completion_and_unknown_stays_unknown(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            first = log.begin('optic_fs_read', 1)
            second = log.begin('optic_task_start', 2)
            self.assertTrue(log.finish(first, {'ok': True}))
            self.assertEqual(ToolCheckpoint(directory).previous['event']['phase'], 'admitted')
            self.assertTrue(log.finish(second, {'ok': False, 'code': 'execution.outcome_unknown'}))
            observed = ToolCheckpoint(directory).previous['event']
            self.assertTrue(observed['outcome_uncertain'])
            self.assertEqual(observed['sequence'], 2)

    def test_valid_interrupted_publication_is_newer_and_fixed_sidecar_is_reused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = ToolCheckpoint(root)
            log.begin('optic_fs_read', 1)
            pending = {**log.current, 'sequence': 2, 'tool': 'optic_fs_write', 'outcome_uncertain': True}
            (root / PENDING).write_text(json.dumps(pending), encoding='utf-8')
            recovered = ToolCheckpoint(root)
            self.assertTrue(recovered.previous['interrupted_publication'])
            self.assertEqual(recovered.previous['event']['sequence'], 2)
            self.assertFalse((root / PENDING).exists())
            self.assertEqual(ToolCheckpoint(root).previous['event']['sequence'], 2)
            self.assertIsNotNone(recovered.begin('optic_fs_read', 1))

    def test_malformed_oversize_duplicate_or_hardlinked_records_never_become_context(self):
        for data in (b'{}', b'x' * (MAX_BYTES + 1), b'{"version":1,"version":1}'):
            with self.subTest(data=data[:20]), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / NAME).write_bytes(data)
                log = ToolCheckpoint(root)
                self.assertEqual(log.previous['state'], 'invalid')
                self.assertTrue(log.failed)
                self.assertIsNone(log.begin('optic_fs_read', 1))
                self.assertEqual((root / NAME).read_bytes(), data)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = ToolCheckpoint(root)
            log.begin('optic_fs_read', 1)
            os.link(root / NAME, root / 'alias')
            self.assertEqual(ToolCheckpoint(root).previous['state'], 'invalid')

    def test_older_ingress_arriving_late_cannot_poison_or_replace_latest_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            self.assertEqual(log.begin('optic_fs_read', 10), 10)
            self.assertIsNone(log.begin('optic_fs_write', 9))
            self.assertFalse(log.failed)
            self.assertEqual(ToolCheckpoint(directory).previous['event']['sequence'], 10)
            self.assertEqual(log.begin('optic_fs_read', 11), 11)

    def test_start_failure_stops_later_admission_without_removing_unknown_sidecar(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = ToolCheckpoint(root)
            (root / PENDING).mkdir()
            self.assertIsNone(log.begin('optic_fs_read', 1))
            self.assertTrue(log.failed)
            self.assertIsNone(log.begin('optic_fs_read', 2))
            self.assertTrue((root / PENDING).is_dir())

    def test_unknown_payload_error_is_sanitized_and_completed_bool_is_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            ticket = log.begin('optic_fs_read', 1)
            self.assertTrue(log.finish(ticket, {'ok': False, 'code': 'SECRET /private/path'}))
            event = ToolCheckpoint(directory).previous['event']
            self.assertEqual(event['code'], 'project.unclassified_error')
            with self.assertRaises(ValueError):
                validate({**event, 'ok': 1})
            with self.assertRaises(ValueError):
                validate({**event, 'source': 'SECRET'})


@asynccontextmanager
async def serving(checkpoint, forward):
    state = Registry()
    owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
    gate = ProjectGate(state, owner, forward, file_budget=ProjectFiles())
    app = make_project_app(gate, state, supervised=True, checkpoint=checkpoint, emit=lambda _: None)
    listener = socket.socket()
    listener.bind(('127.0.0.1', 0))
    listener.listen(32)
    url = f'http://127.0.0.1:{listener.getsockname()[1]}/mcp'
    disabled = logging.root.manager.disable
    logging.disable(logging.CRITICAL)
    service = uvicorn.Server(uvicorn.Config(app, log_config=None, access_log=False, timeout_graceful_shutdown=2))
    task = asyncio.create_task(service.serve(sockets=[listener]))
    try:
        async with asyncio.timeout(5):
            while not service.started:
                if task.done():
                    await task
                await asyncio.sleep(.01)
        async with client(url, TOKENS['A'][0]) as session:
            yield session
    finally:
        service.should_exit = True
        await asyncio.wait_for(task, 5)
        listener.close()
        logging.disable(disabled)


class RetainedIo(unittest.IsolatedAsyncioTestCase):
    async def test_cancelled_waiter_retains_single_io_slot_without_dispatch_queue(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            entered, release = threading.Event(), threading.Event()
            original = log._save
            def delayed(event):
                entered.set()
                if not release.wait(5):
                    raise OSError('test release timeout')
                original(event)
            log._save = delayed
            caller = asyncio.create_task(log.begin_async('optic_fs_write', 1))
            try:
                async with asyncio.timeout(2):
                    while not entered.is_set():
                        await asyncio.sleep(.01)
                caller.cancel()
                with self.assertRaises(asyncio.CancelledError):
                    await caller
                self.assertTrue(log.busy)
                for sequence in range(2, 12):
                    self.assertIsNone(await log.begin_async('optic_fs_read', sequence))
                self.assertFalse(log.failed)
            finally:
                release.set()
                async with asyncio.timeout(2):
                    while log.busy:
                        await asyncio.sleep(.01)
            self.assertEqual(ToolCheckpoint(directory).previous['event']['sequence'], 1)
            self.assertIsNotNone(await log.begin_async('optic_fs_read', 12))


class Http(unittest.IsolatedAsyncioTestCase):
    async def test_read_only_previous_context_survives_new_calls_without_replay(self):
        with tempfile.TemporaryDirectory() as directory:
            first = ToolCheckpoint(directory)
            first.begin('optic_fs_patch', 99)
            current = ToolCheckpoint(directory)
            calls = []
            async def forward(name, arguments):
                calls.append(name)
                return {'capabilities': []}
            async with serving(current, forward) as session:
                for _ in range(2):
                    value = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
                    self.assertEqual(value['previous_tool_checkpoint']['event']['sequence'], 99)
                    self.assertTrue(value['previous_tool_checkpoint']['event']['outcome_uncertain'])
                self.assertEqual(calls, [])
                value = structured(await session.call_tool('optic_project_info', {}))
                self.assertTrue(value['ok'])
                self.assertEqual(calls, ['session_info'])
                value = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
                self.assertEqual(value['previous_tool_checkpoint']['event']['sequence'], 99)

    async def test_checkpoint_failure_before_admission_cannot_dispatch_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            calls = []
            async def forward(name, arguments):
                calls.append(name)
                return {}
            async with serving(log, forward) as session:
                with patch.object(log, '_save', side_effect=OSError('SECRET')):
                    value = structured(await session.call_tool('optic_fs_write', {
                        'path': 'src/a.txt', 'content': 'SECRET', 'expected_kind': 'absent'}))
                self.assertEqual(value['code'], 'project.checkpoint_unavailable')
                self.assertFalse(value['dispatch_attempted'])
                self.assertEqual(value['diagnostic_tool'], 'optic_diagnostics')
                self.assertEqual(calls, [])
                value = structured(await session.call_tool('optic_diagnostics', {}))['upstream']
                self.assertFalse(value['checkpoint_persistence_healthy'])

    async def test_finish_persistence_failure_preserves_confirmed_effect_result(self):
        with tempfile.TemporaryDirectory() as directory:
            log = ToolCheckpoint(directory)
            calls = []
            async def forward(name, arguments):
                calls.append(name)
                # The effect returns successfully; only the metadata write fails.
                log._save = lambda _: (_ for _ in ()).throw(OSError('SECRET'))
                return {'content_version': 'a' * 64}
            async with serving(log, forward) as session:
                value = structured(await session.call_tool('optic_fs_write', {
                    'path': 'src/a.txt', 'content': 'SECRET', 'expected_kind': 'absent'}))
                self.assertTrue(value['ok'])
                self.assertFalse(value['checkpoint_saved'])
                self.assertEqual(value['upstream']['content_version'], 'a' * 64)
                self.assertEqual(calls, ['fs_write'])
                value = structured(await session.call_tool('optic_project_info', {}))
                self.assertEqual(value['code'], 'project.checkpoint_unavailable')
                self.assertEqual(calls, ['fs_write'])
                event = ToolCheckpoint(directory).previous['event']
                self.assertEqual(event['phase'], 'admitted')
                self.assertTrue(event['outcome_uncertain'])
