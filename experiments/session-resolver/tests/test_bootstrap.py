from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import sys
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'credential-lifecycle'))
sys.path.insert(0, str(ROOT / 'francestudent-probe'))
from lifecycle_core import Registry, TOKENS
from bootstrap_core import BootstrapResolver


class Tests(unittest.TestCase):
    def setUp(self):
        self.now = 0
        self.registry = Registry()
        self.a = self.registry.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.b = self.registry.authenticate(b'Bearer ' + TOKENS['B'][0])
        self.resolver = BootstrapResolver(self.registry, clock=lambda: self.now)

    def ticket(self):
        return self.resolver.bootstrap(self.a, 'francestudent')['application_handle']

    def test_lost_reply_concurrent_retry_exactly_one_live_mapping(self):
        ticket = self.ticket()
        with ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(lambda _: self.resolver.open(self.a, 'francestudent', ticket), range(20)))
        self.assertTrue(all(r['ok'] for r in results))
        self.assertEqual(sum(not r['reused'] for r in results), 1)
        self.assertEqual(len({r['application_handle'] for r in results}), 1)
        self.assertEqual(len({r['synthetic_session_id'] for r in results}), 1)
        self.assertEqual(len(self.resolver.sessions), 1)

    def test_repeat_probe_fixed_expiry_and_no_reopen(self):
        ticket = self.ticket()
        first = self.resolver.open(self.a, 'francestudent', ticket)
        self.now = 119
        second = self.resolver.open(self.a, 'francestudent', ticket)
        self.assertEqual(first['application_handle'], second['application_handle'])
        self.assertEqual(second['expires_in_seconds'], 1)
        self.assertTrue(self.resolver.probe(self.a, 'francestudent', second['application_handle'])['ok'])
        self.now = 120
        self.assertFalse(self.resolver.open(self.a, 'francestudent', ticket)['ok'])
        self.assertFalse(self.resolver.probe(self.a, 'francestudent', first['application_handle'])['ok'])

    def test_owner_adapter_and_ticket_session_domains(self):
        ticket = self.ticket()
        for owner, adapter in [(self.b, 'francestudent'), (self.a, 'synthetic-host'), (None, {})]:
            self.assertFalse(self.resolver.open(owner, adapter, ticket)['ok'])
        first = self.resolver.open(self.a, 'francestudent', ticket)
        self.assertFalse(self.resolver.probe(self.a, 'francestudent', ticket)['ok'])
        self.assertFalse(self.resolver.open(self.a, 'francestudent', first['application_handle'])['ok'])
        self.assertFalse(self.resolver.probe(self.b, 'francestudent', first['application_handle'])['ok'])
        self.assertTrue(self.resolver.probe(self.a, 'francestudent', first['application_handle'])['ok'])

    def test_close_and_epoch_cannot_resurrect(self):
        ticket = self.ticket()
        first = self.resolver.open(self.a, 'francestudent', ticket)
        self.resolver.close_owner_local('A')
        self.assertFalse(self.resolver.open(self.a, 'francestudent', ticket)['ok'])
        self.assertFalse(self.resolver.probe(self.a, 'francestudent', first['application_handle'])['ok'])
        ticket = self.ticket()
        self.registry.control('rotate', 'A')
        new = self.registry.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertFalse(self.resolver.open(new, 'francestudent', ticket)['ok'])
        self.assertTrue(self.resolver.bootstrap(new, 'francestudent')['ok'])
        self.registry.control('revoke', 'A')
        self.assertFalse(self.resolver.bootstrap(new, 'francestudent')['ok'])
        self.assertTrue(self.resolver.bootstrap(self.b, 'francestudent')['ok'])

    def test_capacity_retry_and_budget_do_not_evict(self):
        self.resolver = BootstrapResolver(self.registry, clock=lambda: self.now, capacity=1, budget=2)
        one = self.ticket()
        two = self.resolver.bootstrap(self.b, 'francestudent')['application_handle']
        self.assertTrue(self.resolver.open(self.a, 'francestudent', one)['ok'])
        self.assertEqual(self.resolver.open(self.b, 'francestudent', two)['code'], 'probe.session_capacity')
        self.assertTrue(self.resolver.open(self.a, 'francestudent', one)['reused'])
        self.resolver.close_owner_local('A')
        self.assertTrue(self.resolver.open(self.b, 'francestudent', two)['ok'])
        self.assertFalse(self.resolver.bootstrap(self.a, 'francestudent')['ok'])
        self.assertEqual(len(self.resolver.tickets), 2)

    def test_collision_restart_and_digest_only_state(self):
        self.resolver.mint = lambda: b'x' * 32
        ticket = self.ticket()
        self.assertFalse(self.resolver.bootstrap(self.a, 'francestudent')['ok'])
        first = self.resolver.open(self.a, 'francestudent', ticket)
        joined = repr(self.resolver.tickets) + repr(self.resolver.sessions)
        self.assertNotIn(ticket, joined)
        self.assertNotIn(first['application_handle'], joined)
        other = BootstrapResolver(self.registry, clock=lambda: self.now, mint=lambda: b'x' * 32)
        new = other.bootstrap(self.a, 'francestudent')['application_handle']
        self.assertNotEqual(ticket, new)
        self.assertFalse(other.open(self.a, 'francestudent', ticket)['ok'])

    def test_malformed_and_clock_fail_closed(self):
        ticket = self.ticket()
        for handle in [None, {}, 'x' * 100000, ticket.upper()]:
            self.assertFalse(self.resolver.open(self.a, 'francestudent', handle)['ok'])
        self.now = -1
        self.assertFalse(self.resolver.open(self.a, 'francestudent', ticket)['ok'])
        self.now = float('nan')
        self.assertFalse(self.resolver.bootstrap(self.a, 'francestudent')['ok'])

    def test_open_serializes_with_rotation_then_stale_probe_denied(self):
        ticket = self.ticket()
        entered, release, rotating = threading.Event(), threading.Event(), threading.Event()
        def mint_id(_):
            entered.set()
            assert release.wait(2)
            return 'f' * 32
        def rotate():
            rotating.set()
            return self.registry.control('rotate', 'A')
        with patch('bootstrap_core.secrets.token_hex', side_effect=mint_id):
            with ThreadPoolExecutor(max_workers=2) as pool:
                opened = pool.submit(self.resolver.open, self.a, 'francestudent', ticket)
                try:
                    self.assertTrue(entered.wait(2))
                    rotated = pool.submit(rotate)
                    self.assertTrue(rotating.wait(2))
                    self.assertFalse(rotated.done())
                finally:
                    release.set()
                first, ack = opened.result(timeout=2), rotated.result(timeout=2)
        self.assertTrue(first['ok'] and ack['ok'])
        new = self.registry.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertFalse(self.resolver.probe(new, 'francestudent', first['application_handle'])['ok'])
        self.assertFalse(self.resolver.open(new, 'francestudent', ticket)['ok'])


class GuardTests(unittest.IsolatedAsyncioTestCase):
    async def test_explicit_second_handle_tool_does_not_widen_arguments(self):
        import json
        from server import Guard
        from smoke import header_map
        async def downstream(scope, receive, send):
            await send({'type': 'http.response.start', 'status': 200, 'headers': []})
        app = Guard(downstream, emit=lambda _: None, zero_arg_tools=('session_bootstrap',),
                    handle_tool='session_open', extra_handle_tools=('session_probe',))
        headers = [(k.lower().encode(), v.encode()) for k, v in header_map().items()]
        cases = [('session_probe', {'application_handle': 'oph_' + '1' * 64}, 200),
                 ('session_probe', {'application_handle': 'oph_' + '1' * 64, 'project': 'x'}, 400),
                 ('unknown_synthetic_probe', {'application_handle': 'oph_' + '1' * 64}, 400),
                 ('session_probe', {'application_handle': 'x'}, 400)]
        for name, args, status in cases:
            messages = []
            async def receive():
                return {'type': 'http.request', 'body': json.dumps({'method': 'tools/call',
                    'params': {'name': name, 'arguments': args}}).encode()}
            async def send(message):
                messages.append(message)
            await app({'type': 'http', 'method': 'POST', 'path': '/mcp',
                       'query_string': b'', 'headers': headers}, receive, send)
            self.assertEqual(messages[0]['status'], status)


if __name__ == '__main__':
    unittest.main()
