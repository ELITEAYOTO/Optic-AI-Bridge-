"""Offline observer regressions; never launch Bridge or deliver OS signals."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "host_shutdown_supervisor", Path(__file__).with_name("host_shutdown_supervisor.py"))
supervisor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(supervisor)
active = supervisor.active_proof()


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


class ManagedReadyTests(unittest.TestCase):
    def test_partial_record_waits_then_complete_nonce_and_pid_match(self):
        nonce = "a" * 32
        prefix = ("OPTIC_ACTIVE " + nonce + " 123").encode("ascii")
        self.assertIsNone(active.ready_pid(prefix, nonce))
        self.assertEqual(active.ready_pid(prefix + b"\n", nonce), 123)

    def test_foreign_nonce_malformed_or_unbounded_record_fails(self):
        nonce = "a" * 32
        for value in [("OPTIC_ACTIVE " + "b" * 32 + " 123\n").encode("ascii"),
                      b"not a readiness record\n", b"x" * 257,
                      ("OPTIC_ACTIVE " + nonce + " 4294967296\n").encode("ascii")]:
            with self.subTest(value=value), self.assertRaises(AssertionError):
                active.ready_pid(value, nonce)

    def test_program_nonce_and_newline_are_not_injectable_or_double_escaped(self):
        program = active.node_program("a" * 32)
        self.assertIn("process.pid+'\\n'", program)
        self.assertNotIn("\\\\n", program)
        with self.assertRaises(AssertionError):
            active.node_program("injected'code")

    def test_ownership_record_is_published_after_close(self):
        with tempfile.TemporaryDirectory(prefix="optic-active-record-") as directory:
            path = Path(directory) / "owned.json"
            active.publish_json(path, {"pid": 123, "nonce": "a" * 32})
            self.assertEqual(supervisor.json.loads(path.read_text(encoding="utf-8")),
                             {"pid": 123, "nonce": "a" * 32})
            self.assertFalse(path.with_name(path.name + ".pending").exists())


class UnreadMetadataTests(unittest.TestCase):
    def test_fixed_batch_is_bounded_unique_and_metadata_only(self):
        batch = supervisor.unread_metadata_batch()
        self.assertLessEqual(len(batch), 4096)
        self.assertTrue(batch.endswith(b"\n"))
        messages = [supervisor.json.loads(line) for line in batch.splitlines()]
        self.assertEqual(len(messages), 32)
        self.assertEqual({message["id"] for message in messages}, set(range(1000, 1032)))
        for message in messages:
            self.assertEqual(message, {"jsonrpc": "2.0", "id": message["id"],
                "method": "tools/call", "params": {"name": "session_info", "arguments": {}}})


class ExpiryEvidenceTests(unittest.TestCase):
    def test_only_exact_inactive_authority_error_is_a_refusal(self):
        response = {"error": {"code": -32600, "message": "optic.session_inactive"}}
        active.require_inactive_rejection(response)
        for invalid in [{"result": {}}, {"result": {}, **response},
                        {"error": {"code": -32603, "message": "optic.session_inactive"}},
                        {"error": {"code": -32600, "message": "optic.policy_denied"}}]:
            with self.subTest(invalid=invalid), self.assertRaises(AssertionError):
                active.require_inactive_rejection(invalid)

    def test_session_reap_requires_complete_exact_one_session_record(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stderr.log"
            marker = b"Optic AI Bridge session expiry cleanup reaped 1 session(s)"
            for invalid in [marker, marker.replace(b"1", b"2") + b"\n", b"prefix " + marker + b"\n"]:
                path.write_bytes(invalid)
                self.assertFalse(active.session_reaped(path))
            path.write_bytes(marker + b"\r\n")
            self.assertTrue(active.session_reaped(path))

    def test_session_reap_does_not_relax_log_size_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stderr.log"
            path.write_bytes(b"x" * 65537)
            with self.assertRaises(AssertionError):
                active.session_reaped(path)

    def test_raw_rpc_error_observation_keeps_positive_receive_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stdout.log"
            response = {"jsonrpc": "2.0", "id": 7,
                        "error": {"code": -32600, "message": "optic.session_inactive"}}
            path.write_text(supervisor.json.dumps(response) + "\n", encoding="utf-8")
            self.assertEqual(supervisor.receive_response(path, 7, None), response)
            with self.assertRaises(AssertionError):
                supervisor.receive(path, 7, None)

    def test_positive_rpc_response_is_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "stdout.log"
            path.write_text('{"jsonrpc":"2.0","id":7,"result":{"ok":true}}\n', encoding="utf-8")
            self.assertEqual(supervisor.receive(path, 7, None), {"ok": True})


class FailureEvidenceTests(unittest.TestCase):
    def test_missing_log_does_not_mask_original_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = {}
            supervisor.remember_failure(result, root, "active-job", AssertionError("first"))
            self.assertEqual(result["failure"]["message"], "first")
            self.assertTrue(result["failure"]["worker_log_tail"]["missing"])

    def test_oversized_invalid_utf8_tail_is_bounded_without_weakening_log_guard(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "worker.log"
            path.write_bytes(b"x" * supervisor.LIMIT + b"\xff" * supervisor.FAILURE_TAIL_LIMIT)
            with self.assertRaises(AssertionError):
                supervisor.bounded_read(path)
            evidence = supervisor.failure_tail(path)
            self.assertEqual(evidence["bytes_kept"], supervisor.FAILURE_TAIL_LIMIT)
            self.assertEqual(len(evidence["text"]), supervisor.FAILURE_TAIL_LIMIT)
            self.assertTrue(evidence["truncated"])

    def test_first_cause_and_logs_survive_cleanup_and_later_failure(self):
        result = {}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "worker.log").write_bytes(b"worker startup exception")
            (root / "stderr.log").write_bytes(b"original Bridge phase")
            supervisor.remember_failure(result, root, "active-job", AssertionError("ownership"))
            supervisor.remember_failure(result, root, "active-job", OSError("later cleanup"))
        evidence = result["failure"]
        self.assertEqual(evidence["error_type"], "AssertionError")
        self.assertEqual(evidence["message"], "ownership")
        self.assertEqual(evidence["worker_log_tail"]["text"], "worker startup exception")
        self.assertEqual(evidence["bridge_stderr_tail"]["text"], "original Bridge phase")

    def test_unreadable_diagnostic_does_not_replace_case_error(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "worker.log").mkdir()
            result = {}
            supervisor.remember_failure(result, root, "ctrl-break", RuntimeError("deadline"))
            self.assertEqual(result["failure"]["message"], "deadline")
            self.assertIn("read_error", result["failure"]["worker_log_tail"])


if __name__ == "__main__":
    unittest.main()
