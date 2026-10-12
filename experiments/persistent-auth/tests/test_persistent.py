import copy
import json
from pathlib import Path
import tempfile
import queue
import subprocess
import sys
import threading
import unittest
from unittest.mock import patch
from persistent_server import Store, PersistentRegistry, FixtureError
from persistent_core import MAX_STATE, validate
from lifecycle_core import TOKENS


def bearer(owner="A", version=0):
    return b"Bearer " + TOKENS[owner][version]


class PersistentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "fixture"
        Store.initialize(self.root)

    def tearDown(self):
        self.temp.cleanup()

    def test_committed_rotation_and_revoke_survive_reopen(self):
        state = PersistentRegistry(self.root)
        original = state.authenticate(bearer())
        self.assertTrue(state.control("rotate", "A")["persisted"])
        state.close()
        state = PersistentRegistry(self.root)
        try:
            self.assertIsNone(state.authenticate(bearer()))
            new = state.authenticate(bearer(version=1))
            self.assertEqual(new.principal, original.principal)
            self.assertEqual(new.epoch, 2)
            self.assertFalse(state.current(original))
            self.assertEqual(state.control("rotate", "A")["code"], "probe.rotation_limit")
            self.assertTrue(state.control("revoke", "A")["persisted"])
        finally:
            state.close()
        state = PersistentRegistry(self.root)
        try:
            self.assertIsNone(state.authenticate(bearer()))
            self.assertIsNone(state.authenticate(bearer(version=1)))
            self.assertIsNotNone(state.authenticate(bearer("B")))
        finally:
            state.close()

    def test_exclusive_writer_and_release(self):
        state = PersistentRegistry(self.root)
        try:
            with self.assertRaises(FixtureError):
                PersistentRegistry(self.root)
        finally:
            state.close()
        state = PersistentRegistry(self.root)
        state.close()

    def test_missing_state_never_initializes(self):
        (self.root / "state.json").unlink()
        with self.assertRaises(FixtureError):
            PersistentRegistry(self.root)
        self.assertFalse((self.root / "state.json").exists())
        with self.assertRaises(FixtureError):
            Store.initialize(self.root)

    def test_corrupt_duplicate_oversize_and_unknown_schema_fail_closed(self):
        for raw in (b"", b"{", b"x" * (MAX_STATE + 1),
                    b'{"schema":1,"schema":1,"owners":{}}',
                    b'{"schema":2,"owners":{}}'):
            (self.root / "state.json").write_bytes(raw)
            with self.assertRaises(FixtureError):
                PersistentRegistry(self.root)

    def test_exact_schema_and_no_raw_credentials(self):
        store = Store(self.root)
        try:
            owners = store.read()
            for mutation in (lambda x: x["A"].update(active="yes"),
                             lambda x: x["A"].update(epoch=True),
                             lambda x: x["A"].update(epoch=65536),
                             lambda x: x["A"].update(principal="private-canary"),
                             lambda x: x["A"].update(version=1),
                             lambda x: x["A"].update(token="private-canary"),
                             lambda x: x["B"].update(principal=x["A"]["principal"])):
                candidate = copy.deepcopy(owners)
                mutation(candidate)
                with self.assertRaises(FixtureError):
                    validate({"schema": 1, "owners": candidate})
            text = (self.root / "state.json").read_text()
            self.assertTrue(all(value.decode() not in text for tokens in TOKENS.values() for value in tokens))
            self.assertLessEqual(len(text.encode()), MAX_STATE)
        finally:
            store.close()

    def test_commit_failure_blocks_without_echo_or_success_ack(self):
        state = PersistentRegistry(self.root)
        try:
            with patch("os.replace", side_effect=OSError("PRIVATE-COMMIT-CANARY")):
                result = state.control("revoke", "A")
            self.assertFalse(result["ok"])
            self.assertNotIn("PRIVATE-COMMIT-CANARY", repr(result))
            self.assertIsNone(state.authenticate(bearer()))
            self.assertIsNone(state.authenticate(bearer("B")))
            self.assertEqual(state.control("rotate", "B")["code"], "fixture.blocked")
        finally:
            state.close()
        # Failed/unacknowledged revoke is not promised committed: last valid
        # on-disk state remains the operator's recovery reference.
        state = PersistentRegistry(self.root)
        try:
            self.assertIsNotNone(state.authenticate(bearer()))
            self.assertTrue(state.control("rotate", "A")["ok"])
            self.assertFalse((self.root / "state.pending").exists())
        finally:
            state.close()

    def test_pending_state_never_overrides_committed_state(self):
        state = PersistentRegistry(self.root)
        try:
            pending = copy.deepcopy(state.states)
            pending["A"].update(active=False, epoch=2)
        finally:
            state.close()
        (self.root / "state.pending").write_text(json.dumps({"schema": 1, "owners": pending}))
        state = PersistentRegistry(self.root)
        try:
            self.assertIsNotNone(state.authenticate(bearer()))
            self.assertTrue(state.control("revoke", "A")["persisted"])
            self.assertFalse((self.root / "state.pending").exists())
        finally:
            state.close()

    def test_symlink_state_rejected_if_supported(self):
        path = self.root / "state.json"
        target = self.root / "other.json"
        path.rename(target)
        try:
            path.symlink_to(target)
        except OSError:
            self.skipTest("native symlink permission unavailable")
        with self.assertRaises(FixtureError):
            PersistentRegistry(self.root)

    def test_actual_crash_before_replace_does_not_promote_pending(self):
        helper = '''
import sys, threading
from persistent_server import PersistentRegistry
import persistent_core
state = PersistentRegistry(sys.argv[1])
def paused_replace(*args):
    print("BEFORE_REPLACE", flush=True)
    threading.Event().wait()
persistent_core.os.replace = paused_replace
state.control("revoke", "A")
'''
        process = subprocess.Popen([sys.executable, "-c", helper, str(self.root)],
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        ready = queue.Queue(maxsize=1)
        reader = threading.Thread(target=lambda: ready.put(process.stdout.readline()), daemon=True)
        reader.start()
        try:
            self.assertEqual(ready.get(timeout=10).strip(), "BEFORE_REPLACE")
        finally:
            process.kill()
            process.wait(timeout=5)
            reader.join(timeout=5)
            process.stdout.close()
        self.assertTrue((self.root / "state.pending").exists())
        state = PersistentRegistry(self.root)
        try:
            self.assertIsNotNone(state.authenticate(bearer()))
            self.assertEqual(state.authenticate(bearer()).epoch, 1)
            self.assertTrue(state.control("revoke", "A")["persisted"])
        finally:
            state.close()

    def test_live_missing_state_blocks_commit(self):
        state = PersistentRegistry(self.root)
        try:
            (self.root / "state.json").unlink()
            self.assertFalse(state.control("rotate", "A")["ok"])
            self.assertIsNone(state.authenticate(bearer()))
            self.assertFalse((self.root / "state.json").exists())
        finally:
            state.close()

    def test_live_corrupt_state_is_not_overwritten(self):
        state = PersistentRegistry(self.root)
        try:
            path = self.root / "state.json"
            path.write_bytes(b"PRIVATE-CORRUPT-CANARY")
            result = state.control("revoke", "A")
            self.assertFalse(result["ok"])
            self.assertNotIn("PRIVATE-CORRUPT-CANARY", repr(result))
            self.assertIsNone(state.authenticate(bearer()))
            self.assertEqual(path.read_bytes(), b"PRIVATE-CORRUPT-CANARY")
        finally:
            state.close()


if __name__ == "__main__":
    unittest.main()
