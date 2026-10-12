import asyncio
from concurrent.futures import ThreadPoolExecutor
import json
import unittest

from probe_core import DUMMY_HEADERS, DUMMY_TOKENS, Handles, observe_headers
from server import Guard, MAX_BODY, MAX_HEADERS, MAX_REQUESTS, MAX_LIFETIME, request_proof


def headers(owner="A"):
    return [(b"authorization", b"Bearer " + DUMMY_TOKENS[owner]), *DUMMY_HEADERS.items()]


class HandleTests(unittest.TestCase):
    def test_single_use_owner_and_exact_bytes(self):
        store = Handles()
        handle = store.create("A")["application_handle"]
        self.assertFalse(store.consume("B", handle)["ok"])
        self.assertFalse(store.consume("A", handle + "x")["ok"])
        self.assertTrue(store.consume("A", handle)["passed_unchanged"])
        self.assertFalse(store.consume("A", handle)["ok"])
        self.assertEqual(store.entries, {})

    def test_capacity_expiry_and_restart(self):
        now = [0]
        store = Handles(clock=lambda: now[0], capacity=2)
        first = store.create("A")["application_handle"]
        store.create("A")
        self.assertEqual(store.create("A")["code"], "probe.capacity")
        now[0] = 120
        self.assertFalse(store.consume("A", first)["ok"])
        self.assertTrue(store.create("B")["ok"])
        self.assertFalse(Handles().consume("A", first)["ok"])

    def test_atomic_race_and_collision(self):
        store = Handles()
        handle = store.create("A")["application_handle"]
        with ThreadPoolExecutor(max_workers=8) as pool:
            outcomes = list(pool.map(lambda _: store.consume("A", handle)["ok"], range(16)))
        self.assertEqual(sum(outcomes), 1)
        same = Handles(mint=lambda: "oph_" + "a" * 64)
        self.assertTrue(same.create("A")["ok"])
        self.assertFalse(same.create("B")["ok"])
        self.assertTrue(same.consume("A", "oph_" + "a" * 64)["ok"])

    def test_unknown_owner_and_bounded_keys(self):
        store = Handles()
        self.assertFalse(store.create("forged")["ok"])
        for _ in range(32):
            self.assertTrue(store.create("A")["ok"])
        self.assertFalse(store.create("B")["ok"])
        self.assertTrue(all(isinstance(k, bytes) and len(k) == 32 for k in store.entries))


class HeaderTests(unittest.TestCase):
    def test_case_duplicates_malformed_and_redaction(self):
        canary = b"REAL-BEARER-CANARY-never-log"
        for h in ([], [(b"Authorization", canary)], headers() + [(b"authorization", canary)],
                  [(b"authorization", b"bearer " + DUMMY_TOKENS["A"])],
                  [(b"authorization", b"Bearer  " + DUMMY_TOKENS["A"])]):
            owner, observation = observe_headers(h)
            self.assertIsNone(owner)
            self.assertNotIn(canary.decode(), json.dumps(observation))
        owner, observation = observe_headers([(k.upper(), v) for k, v in headers()])
        self.assertEqual(owner, "A")
        self.assertTrue(all(v["matches_dummy"] for v in observation["custom"].values()))
        _, observation = observe_headers(headers() + [(b"x-optic-probe", canary)])
        self.assertFalse(observation["custom"]["x-optic-probe"]["matches_dummy"])


class GuardTests(unittest.IsolatedAsyncioTestCase):
    async def request(self, guard, h=None, body=b"", **changes):
        output = []
        scope = {"type": "http", "path": "/mcp", "query_string": b"", "headers": headers() if h is None else h}
        scope.update(changes)
        async def receive():
            return {"type": "http.request", "body": body, "more_body": False}
        async def send(message):
            output.append(message)
        await guard(scope, receive, send)
        return output

    async def test_per_request_fail_closed_and_no_canary(self):
        logs, entered = [], []
        async def app(scope, receive, send):
            entered.append(request_proof.get())
            await Guard.reject(send, 200)
        guard = Guard(app, emit=logs.append)
        canary = b"REAL-BEARER-CANARY-never-log"
        for h, status in ((headers(), 200), ([], 401),
                ([(b"authorization", b"Bearer " + canary)], 401),
                (headers() + [(b"authorization", canary)], 401),
                (headers()[:-1], 401), (headers("B"), 200)):
            output = await self.request(guard, h)
            self.assertEqual(output[0]["status"], status)
            self.assertNotIn(canary.decode(), repr(output) + repr(logs))
        self.assertEqual([x["owner"] for x in entered], ["A", "B"])
        self.assertEqual(guard.active, 0)
        self.assertIsNone(request_proof.get())
        self.assertEqual(sum(" HTTP " in line for line in logs), 6)
        self.assertEqual(sum(" STATUS " in line for line in logs), 6)

    async def test_bounds_and_unknown_tool(self):
        async def app(*_):
            self.fail("denied request dispatched")
        guard = Guard(app, emit=lambda _: None)
        cases = [({"body": b"x" * (MAX_BODY + 1)}, 413),
                 ({"h": headers() + [(b"extra", b"x" * MAX_HEADERS)]}, 431),
                 ({"query_string": b"token=canary"}, 400),
                 ({"body": b'{"method":"tools/call","params":{"name":"session_cancel"}}'}, 400),
                 ({"body": b'{"method":"tools/call","params":{"name":"probe_headers","arguments":{"secret":"canary"}}}'}, 400)]
        for kwargs, status in cases:
            self.assertEqual((await self.request(guard, **kwargs))[0]["status"], status)
        guard.active = 8
        self.assertEqual((await self.request(guard))[0]["status"], 503)
        guard.active = 0
        guard.sequence = MAX_REQUESTS
        self.assertEqual((await self.request(guard))[0]["status"], 429)
        now = [0]
        guard = Guard(app, clock=lambda: now[0], emit=lambda _: None)
        now[0] = MAX_LIFETIME
        self.assertEqual((await self.request(guard))[0]["status"], 429)

    async def test_context_isolation_and_cancellation(self):
        both = asyncio.Event()
        seen = []
        async def app(scope, receive, send):
            before = request_proof.get()
            seen.append(before["owner"])
            if len(seen) == 2:
                both.set()
            await both.wait()
            self.assertIs(request_proof.get(), before)
            await Guard.reject(send, 200)
        guard = Guard(app, emit=lambda _: None)
        await asyncio.gather(self.request(guard, headers()), self.request(guard, headers("B")))
        self.assertCountEqual(seen, ["A", "B"])
        self.assertEqual(guard.active, 0)
        started = asyncio.Event()
        async def blocked(*args):
            started.set()
            await asyncio.Future()
        guard = Guard(blocked, emit=lambda _: None)
        task = asyncio.create_task(self.request(guard))
        await started.wait()
        task.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await task
        self.assertEqual(guard.active, 0)


if __name__ == "__main__":
    unittest.main()
