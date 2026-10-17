import json
from pathlib import Path
import tempfile
import unittest

from stop_report import write_stop_report


class StopReport(unittest.TestCase):
    def test_retained_report_separates_revoke_from_output_and_refuses_false_success(self):
        snapshot = {'accepted': 12, 'delivered': 4, 'dropped': 2, 'queued': 7, 'failed': False}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = write_stop_report(root, {'ok': True, 'session_revoked': True, 'drained': True},
                                       'expiry', False, snapshot)
            self.assertTrue(report['revocation_confirmed'])
            self.assertFalse(report['diagnostics_drained'])
            self.assertEqual(json.loads((root / 'last-stop.json').read_bytes()), report)
            report = write_stop_report(root, {}, 'service_exit', True, snapshot)
            self.assertFalse(report['revocation_confirmed'])
            self.assertFalse(report['work_drained'])
            self.assertEqual([p.name for p in root.iterdir()], ['last-stop.json'])
            with self.assertRaises(ValueError):
                write_stop_report(root, {}, 'SECRET', True, snapshot)
            self.assertEqual(json.loads((root / 'last-stop.json').read_bytes()), report)
