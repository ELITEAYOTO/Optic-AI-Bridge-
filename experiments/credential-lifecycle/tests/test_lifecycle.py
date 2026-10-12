import asyncio
from contextlib import redirect_stdout
from concurrent.futures import ThreadPoolExecutor
import io
import unittest
from unittest.mock import patch
from lifecycle_core import Registry, TOKENS
from lifecycle_server import Recheck, observation, local_commands
from server import Guard, request_proof
from probe_core import DUMMY_HEADERS


def bearer(owner="A", version=0):
    return b"Bearer " + TOKENS[owner][version]


class RegistryTests(unittest.TestCase):
    def test_revoke_rotate_epochs_and_owner(self):
        state = Registry()
        old = state.authenticate(bearer())
        other = state.authenticate(bearer("B"))
        self.assertIsNotNone(old)
        state.control("revoke", "A")
        self.assertIsNone(state.authenticate(bearer()))
        self.assertFalse(state.current(old))
        self.assertTrue(state.current(other))
        self.assertTrue(state.control("rotate", "A")["ok"])
        new = state.authenticate(bearer(version=1))
        self.assertEqual(new.principal, old.principal)
        self.assertGreater(new.epoch, old.epoch)
        self.assertIsNone(state.authenticate(bearer()))
        self.assertFalse(state.current(old))
        self.assertEqual(state.control("rotate", "A")["code"], "probe.rotation_limit")
        state.control("revoke", "A")
        self.assertFalse(state.current(new))
        self.assertIsNone(state.authenticate(bearer(version=1)))

    def test_invalid_and_atomic_rotation(self):
        state = Registry()
        for value in (b"", b"Bearer wrong", TOKENS["A"][0], b"bearer " + TOKENS["A"][0]):
            self.assertIsNone(state.authenticate(value))
        self.assertFalse(state.control("restore", "A")["ok"])
        self.assertFalse(state.control("rotate", "forged")["ok"])
        with ThreadPoolExecutor(max_workers=8) as pool:
            outcomes = list(pool.map(lambda _: state.control("rotate", "A")["ok"], range(16)))
        self.assertEqual(sum(outcomes), 1)
        self.assertEqual(len(state.states), 2)
        self.assertIsNone(state.authenticate(bearer()))
        self.assertIsNotNone(state.authenticate(bearer(version=1)))

    def test_console_never_echoes_unknown_or_oversized_input(self):
        state = Registry()
        for text in ("PRIVATE-CONSOLE-CANARY\n" + "rotate-a\n" * 20,
                     "PRIVATE-CONSOLE-CANARY" * 20 + "\nrotate-a\n"):
            output = io.StringIO()
            with patch("lifecycle_server.registry", state), patch("sys.stdin", io.StringIO(text)), redirect_stdout(output):
                local_commands()
            self.assertNotIn("PRIVATE-CONSOLE-CANARY", output.getvalue())
            self.assertLessEqual(len(output.getvalue().splitlines()), 17)


class DispatchTests(unittest.IsolatedAsyncioTestCase):
    async def test_revoked_after_initial_auth_cannot_dispatch(self):
        state = Registry()
        identity = state.authenticate(bearer())
        output, called = [], []
        async def app(*args):
            called.append(True)
        async def send(message):
            output.append(message)
        token = request_proof.set({"owner": identity})
        try:
            state.control("rotate", "A")
            await Recheck(app, state)({"type": "http"}, None, send)
        finally:
            request_proof.reset(token)
        self.assertEqual(output[0]["status"], 401)
        self.assertEqual(called, [])
        self.assertNotIn(TOKENS["A"][0].decode(), repr(output))

    async def test_rotate_while_body_waits_is_denied_before_mcp(self):
        state = Registry()
        entered, release = asyncio.Event(), asyncio.Event()
        output, calls = [], []
        async def app(*args):
            calls.append(True)
        async def receive():
            entered.set()
            await release.wait()
            return {"type": "http.request", "body": b'{"method":"tools/call","params":{"name":"credential_probe","arguments":{}}}'}
        async def send(message):
            output.append(message)
        gate = Guard(Recheck(app, state), header_observer=observation,
                     zero_arg_tools=("credential_probe",), handle_tool=None, emit=lambda _: None)
        with patch("lifecycle_server.registry", state):
            task = asyncio.create_task(gate({"type": "http", "method": "POST", "path": "/mcp",
                "headers": [(b"authorization", bearer()), *DUMMY_HEADERS.items()]}, receive, send))
            await entered.wait()
            state.control("rotate", "A")
            release.set()
            await task
        self.assertEqual(output[0]["status"], 401)
        self.assertEqual(calls, [])
        self.assertEqual(gate.active, 0)


if __name__ == "__main__":
    unittest.main()
