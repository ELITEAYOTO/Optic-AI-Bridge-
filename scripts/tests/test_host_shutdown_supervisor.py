"""Offline observer regressions; never launch Bridge or deliver OS signals."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "host_shutdown_supervisor", Path(__file__).with_name("host_shutdown_supervisor.py"))
supervisor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(supervisor)


class TimingRecordTests(unittest.TestCase):
    def test_partial_prefix_and_token_are_not_completed_phases(self):
        with tempfile.TemporaryDirectory(prefix="optic-timing-parser-") as directory:
            path = Path(directory) / "stderr.log"
            for fragment in [b"OPTIC_STARTUP phase=", b"OPTIC_STARTUP phase=host-stop-observed",
                             b"OPTIC_STARTUP phase=host-stop-observed elapsed_ms=3"]:
                path.write_bytes(fragment)
                self.assertEqual(supervisor.phases(path), [])
            with path.open("ab") as stream:
                stream.write(b"0\n")
            self.assertEqual(supervisor.phases(path), ["host-stop-observed"])

    def test_complete_order_is_preserved_before_a_partial_tail(self):
        with tempfile.TemporaryDirectory(prefix="optic-timing-parser-") as directory:
            path = Path(directory) / "stderr.log"
            path.write_bytes(b"OPTIC_STARTUP phase=host-stop-observed elapsed_ms=30\n"
                             b"OPTIC_STARTUP phase=original-session-cleanup-complete elapsed_ms=31\r\n"
                             b"OPTIC_STARTUP phase=")
            self.assertEqual(supervisor.phases(path),
                             ["host-stop-observed", "original-session-cleanup-complete"])

    def test_malformed_complete_record_is_an_error(self):
        with tempfile.TemporaryDirectory(prefix="optic-timing-parser-") as directory:
            path = Path(directory) / "stderr.log"
            path.write_bytes(b"OPTIC_STARTUP phase=\n")
            with self.assertRaises(AssertionError):
                supervisor.phases(path)


if __name__ == "__main__":
    unittest.main()
