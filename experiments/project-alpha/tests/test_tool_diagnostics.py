import json
from types import SimpleNamespace
import unittest

from diagnostics import ToolDiagnostics, DIAGNOSTIC_TOOL, MAX_EVENTS, PAGE_EVENTS


class Diagnostics(unittest.TestCase):
    def test_delayed_old_epoch_reply_cannot_reset_new_epoch_diagnostics(self):
        from lifecycle_core import Registry, TOKENS
        state = Registry()
        first = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        gate = SimpleNamespace(registry=state, stopped=False, owner=first.owner, principal=first.principal)
        log = ToolDiagnostics()
        forged = SimpleNamespace(owner=first.owner, principal=first.principal, epoch=first.epoch)
        self.assertIsNone(log.snapshot_current(gate, None, forged, False, 0))
        log.record_current(gate, None, first, 'optic_fs_read', {'ok': True}, 1, 0, .1)
        state.control('rotate', 'A')
        second = state.authenticate(b'Bearer ' + TOKENS['A'][1])
        log.record_current(gate, None, second, 'optic_fs_read', {'ok': True}, 2, 0, .1)
        log.record_current(gate, None, first, 'optic_fs_read', {'ok': False, 'code': 'link.inactive'}, 3, 0, .1)
        self.assertIsNone(log.snapshot_current(gate, None, first, False, 0))
        result = log.snapshot_current(gate, None, second, False, 0)
        self.assertEqual([event['sequence'] for event in result['events']], [2])
        state.control('revoke', 'A')
        self.assertIsNone(log.snapshot_current(gate, None, second, False, 0))

    def test_bounded_safe_outcomes_and_rotation_clears_history(self):
        owner = SimpleNamespace(principal='owner-a', epoch=1)
        log = ToolDiagnostics()
        for sequence in range(100):
            log.record(owner, 'optic_fs_write', {'ok': False, 'code': 'project.outcome_unknown',
                'content': 'secret source', 'application_handle': 'secret handle'}, sequence, 0, .01)
        result = log.snapshot(owner, True, 2)
        self.assertEqual(len(log.events), MAX_EVENTS)
        self.assertEqual(len(result['events']), PAGE_EVENTS)
        self.assertEqual(result['evicted'], 68)
        self.assertTrue(result['events'][-1]['outcome_uncertain'])
        self.assertTrue(result['pending_forwarding'])
        self.assertNotIn('secret', json.dumps(result))
        self.assertLess(len(json.dumps(result).encode()), 4096)
        log.record(owner, 'optic_fs_read', {'ok': False, 'code': 'raw exception /secret/path'}, 101, 0, 1)
        self.assertEqual(log.events[-1]['code'], 'project.unclassified_error')
        before = log.recorded
        log.record(owner, DIAGNOSTIC_TOOL, {'ok': True}, 102, 0, 1)
        self.assertEqual(log.recorded, before)
        rotated = SimpleNamespace(principal='owner-a', epoch=2)
        self.assertEqual(log.snapshot(rotated, False, 0)['events'], [])
        self.assertEqual(log.recorded, 0)
