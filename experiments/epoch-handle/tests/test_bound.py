from concurrent.futures import ThreadPoolExecutor
import threading
import unittest
from bound_server import EpochHandles
from lifecycle_core import Identity, Registry, TOKENS


def auth(state, owner="A", version=0):
    return state.authenticate(b"Bearer " + TOKENS[owner][version])


class BoundTests(unittest.TestCase):
    def test_exact_owner_single_use_and_replay(self):
        state = Registry()
        a, b = auth(state), auth(state, "B")
        handles = EpochHandles(state)
        value = handles.create(a)["application_handle"]
        self.assertFalse(handles.consume(b, value)["ok"])
        self.assertFalse(handles.consume(a, value + "x")["ok"])
        self.assertTrue(handles.consume(a, value)["passed_unchanged"])
        self.assertFalse(handles.consume(a, value)["ok"])
        self.assertEqual(handles.entries, {})
        self.assertEqual(len(handles.minted), 1)

    def test_rotation_and_revocation_invalidate_bindings(self):
        state = Registry()
        handles = EpochHandles(state)
        a, b = auth(state), auth(state, "B")
        old = handles.create(a)["application_handle"]
        b_value = handles.create(b)["application_handle"]
        state.control("rotate", "A")
        rotated = auth(state, version=1)
        self.assertEqual(rotated.principal, a.principal)
        self.assertGreater(rotated.epoch, a.epoch)
        self.assertEqual(handles.consume(rotated, old)["code"], "probe.invalid_handle")
        self.assertEqual(handles.consume(a, old)["code"], "probe.authentication_inactive")
        self.assertTrue(handles.consume(b, b_value)["ok"])
        new = handles.create(rotated)["application_handle"]
        state.control("revoke", "A")
        self.assertFalse(handles.consume(rotated, new)["ok"])
        self.assertFalse(handles.create(rotated)["ok"])
        self.assertFalse(handles.consume(b, new)["ok"])
        self.assertEqual(handles.entries, {})

    def test_ttl_capacity_reaping_and_restart(self):
        now = [0]
        state = Registry()
        a, b = auth(state), auth(state, "B")
        handles = EpochHandles(state, clock=lambda: now[0], capacity=2)
        first = handles.create(a)["application_handle"]
        handles.create(b)
        self.assertEqual(handles.create(a)["code"], "probe.capacity")
        now[0] = 120
        self.assertFalse(handles.consume(a, first)["ok"])
        self.assertTrue(handles.create(b)["ok"])
        restarted = Registry()
        self.assertFalse(EpochHandles(restarted).consume(auth(restarted), first)["ok"])

    def test_collision_tombstone_and_finite_mint_budget(self):
        state = Registry()
        a = auth(state)
        value = "oph_" + "a" * 64
        handles = EpochHandles(state, mint=lambda: value)
        self.assertTrue(handles.create(a)["ok"])
        self.assertTrue(handles.consume(a, value)["ok"])
        self.assertEqual(handles.create(a)["code"], "probe.mint_failed")
        self.assertFalse(handles.consume(a, value)["ok"])
        bounded = EpochHandles(state, mint_budget=2)
        for _ in range(2):
            token = bounded.create(a)["application_handle"]
            self.assertTrue(bounded.consume(a, token)["ok"])
        self.assertEqual(bounded.create(a)["code"], "probe.mint_budget")
        self.assertEqual(len(bounded.minted), 2)
        self.assertTrue(all(isinstance(k, bytes) and len(k) == 32 for k in bounded.minted))

    def test_atomic_consume_and_forged_identity(self):
        state = Registry()
        a = auth(state)
        handles = EpochHandles(state)
        value = handles.create(a)["application_handle"]
        self.assertFalse(handles.consume(Identity("A", "forged", a.epoch), value)["ok"])
        self.assertFalse(handles.consume(Identity("forged", a.principal, a.epoch), value)["ok"])
        with ThreadPoolExecutor(max_workers=8) as pool:
            outcomes = list(pool.map(lambda _: handles.consume(a, value)["ok"], range(16)))
        self.assertEqual(sum(outcomes), 1)

    def test_rotate_serializes_with_complete_create(self):
        state = Registry()
        a = auth(state)
        entered, release, attempting = threading.Event(), threading.Event(), threading.Event()
        def mint():
            entered.set()
            if not release.wait(timeout=5):
                raise RuntimeError("test release deadline")
            return "oph_" + "b" * 64
        handles = EpochHandles(state, mint=mint)
        def rotate():
            attempting.set()
            return state.control("rotate", "A")
        with ThreadPoolExecutor(max_workers=2) as pool:
            create = pool.submit(handles.create, a)
            try:
                self.assertTrue(entered.wait(timeout=5))
                control = pool.submit(rotate)
                self.assertTrue(attempting.wait(timeout=5))
                self.assertFalse(control.done())
            finally:
                release.set()
            created = create.result(timeout=5)
            self.assertTrue(control.result(timeout=5)["ok"])
        self.assertTrue(created["ok"])
        self.assertFalse(handles.consume(auth(state, version=1), created["application_handle"])["ok"])

    def test_revoke_waits_for_atomic_consume_and_denies_later_work(self):
        state = Registry()
        a = auth(state)
        entered, release, attempting = threading.Event(), threading.Event(), threading.Event()
        block = [False]
        def clock():
            if block[0]:
                entered.set()
                if not release.wait(timeout=5):
                    raise RuntimeError("test release deadline")
            return 0
        handles = EpochHandles(state, clock=clock)
        value = handles.create(a)["application_handle"]
        block[0] = True
        def revoke():
            attempting.set()
            return state.control("revoke", "A")
        with ThreadPoolExecutor(max_workers=2) as pool:
            consume = pool.submit(handles.consume, a, value)
            try:
                self.assertTrue(entered.wait(timeout=5))
                control = pool.submit(revoke)
                self.assertTrue(attempting.wait(timeout=5))
                self.assertFalse(control.done())
            finally:
                release.set()
            self.assertTrue(consume.result(timeout=5)["ok"])
            self.assertTrue(control.result(timeout=5)["ok"])
        self.assertEqual(handles.entries, {})
        self.assertFalse(handles.consume(a, value)["ok"])
        self.assertFalse(handles.create(a)["ok"])


if __name__ == "__main__":
    unittest.main()
