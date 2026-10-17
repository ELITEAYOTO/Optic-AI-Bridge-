import tempfile
from pathlib import Path
import unittest

from node_profiles import configure_node, SOURCE_FILES
from project_resume import select_run, session_lock, previous_session


class Resume(unittest.TestCase):
    def test_previous_stop_summary_is_bounded_filtered_and_never_claims_an_unobserved_crash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(previous_session(root, True), {'state': 'no_previous_session'})
            self.assertEqual(previous_session(root, False), {'state': 'previous_shutdown_unconfirmed'})
            report = root / 'last-stop.json'
            report.write_text('{"version":1,"reason":"operator","revocation_confirmed":true,"work_drained":true,"raw":"secret"}')
            with session_lock(root) as previous:
                self.assertEqual(previous, {'state': 'previous_stop_report_available', 'reason': 'operator',
                    'revocation_confirmed': True, 'work_drained': True})
                self.assertFalse(report.exists())
            with session_lock(root) as previous:
                self.assertEqual(previous, {'state': 'previous_shutdown_unconfirmed'})
            report.write_bytes(b'x' * 1025)
            self.assertEqual(previous_session(root, False), {'state': 'previous_stop_report_invalid'})

    def test_sources_preserved_and_concurrent_session_denied(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            root = output / 'alpha-public'
            (root / 'project/src').mkdir(parents=True)
            (root / 'journal').mkdir()
            node = output / 'node.exe'
            node.write_bytes(b'placeholder, never executed')
            configure_node(root / 'project', node)
            (root / 'project/src/model.js').write_text('module.exports.value = 42;\n')
            before = {p: (root / 'project' / p).read_bytes() for p in SOURCE_FILES}
            selected = select_run(output, root.name)
            self.assertEqual(selected[0], root.resolve(strict=True))
            (root / 'viewer-ready.json').write_text('{}')
            (root / 'last-stop.json').write_text('{"old":true}')
            with session_lock(root):
                self.assertFalse((root / 'viewer-ready.json').exists())
                self.assertFalse((root / 'last-stop.json').exists())
                with self.assertRaises(OSError):
                    with session_lock(root):
                        self.fail('second editor admitted')
                configure_node(selected[1], node, resume=True)
            with session_lock(root):
                configure_node(selected[1], node, resume=True)
            self.assertEqual(before, {p: (root / 'project' / p).read_bytes() for p in SOURCE_FILES})

    def test_named_run_only_and_invalid_sources_fail_without_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for name in ('../alpha-public', str(output), 'alpha-../other', ''):
                with self.assertRaises(ValueError):
                    select_run(output, name)
            root = output / 'alpha-public'
            (root / 'project/src').mkdir(parents=True)
            (root / 'journal').mkdir()
            node = output / 'node.exe'
            node.write_bytes(b'placeholder')
            configure_node(root / 'project', node)
            source = root / 'project/src/model.js'
            for raw in (b'x' * 8193, b'\xff', b''):
                source.write_bytes(raw)
                with self.assertRaises((ValueError, UnicodeError)):
                    configure_node(root / 'project', node, resume=True)
                self.assertEqual(source.read_bytes(), raw)
