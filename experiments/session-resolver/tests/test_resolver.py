from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'credential-lifecycle'))
from lifecycle_core import Identity, Registry, TOKENS
from resolver_core import Resolver


class Clock:
    value = 0
    def __call__(self):
        return self.value


class Tests(unittest.TestCase):
    def setUp(self):
        self.registry, self.clock = Registry(), Clock()
        self.resolver = Resolver(self.registry, clock=self.clock)
        self.a = self.registry.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.b = self.registry.authenticate(b'Bearer ' + TOKENS['B'][0])

    def create(self):
        return self.resolver.open(self.a, 'francestudent')['application_handle']

    def test_repeated_resolution_stable_and_deadline_not_extended(self):
        handle = self.create()
        first = self.resolver.resolve(self.a, 'francestudent', handle)
        self.clock.value = 119
        self.assertIs(first, self.resolver.resolve(self.a, 'francestudent', handle))
        self.assertEqual(first.deadline, 120)
        self.clock.value = 120
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', handle))

    def test_owner_adapter_and_malformed_denial_preserve_session(self):
        handle = self.create()
        for owner, adapter, token in [(self.b, 'francestudent', handle),
                                      (self.a, 'synthetic-host', handle),
                                      (None, 'francestudent', handle),
                                      (self.a, 'francestudent', handle.upper()),
                                      (self.a, {}, handle),
                                      (self.a, 'francestudent', 'x' * 100000)]:
            self.assertIsNone(self.resolver.resolve(owner, adapter, token))
            self.assertFalse(self.resolver.close_local(owner, adapter, token)['ok'])
        self.assertIsNotNone(self.resolver.resolve(self.a, 'francestudent', handle))

    def test_local_close_replay_and_collision_no_recycling(self):
        self.resolver.mint = lambda: '1' * 64
        handle = self.create()
        self.assertTrue(self.resolver.close_local(self.a, 'francestudent', handle)['ok'])
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', handle))
        self.assertFalse(self.resolver.open(self.a, 'francestudent')['ok'])

    def test_rotation_revoke_and_new_handle(self):
        handle = self.create()
        self.registry.control('rotate', 'A')
        new = self.registry.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertIsNone(self.resolver.resolve(new, 'francestudent', handle))
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', handle))
        fresh = self.resolver.open(new, 'francestudent')['application_handle']
        self.assertIsNotNone(self.resolver.resolve(new, 'francestudent', fresh))
        self.registry.control('revoke', 'A')
        self.assertIsNone(self.resolver.resolve(new, 'francestudent', fresh))
        self.assertTrue(self.resolver.open(self.b, 'francestudent')['ok'])

    def test_capacity_no_eviction_and_lifetime_mint_budget(self):
        self.resolver = Resolver(self.registry, clock=self.clock, capacity=1, mint_budget=2)
        first = self.create()
        self.assertFalse(self.resolver.open(self.a, 'francestudent')['ok'])
        self.assertIsNotNone(self.resolver.resolve(self.a, 'francestudent', first))
        self.clock.value = 120
        second = self.create()
        self.assertTrue(self.resolver.close_local(self.a, 'francestudent', second)['ok'])
        self.assertFalse(self.resolver.open(self.a, 'francestudent')['ok'])
        self.assertEqual(len(self.resolver.issued), 2)

    def test_restart_generation_rejects_old_even_with_same_mint(self):
        self.resolver.mint = lambda: '2' * 64
        old = self.create()
        other = Resolver(self.registry, clock=self.clock, mint=lambda: '2' * 64)
        fresh = other.open(self.a, 'francestudent')['application_handle']
        self.assertNotEqual(old, fresh)
        self.assertIsNone(other.resolve(self.a, 'francestudent', old))

    def test_concurrent_opens_have_fixed_capacity_and_distinct_sessions(self):
        self.resolver = Resolver(self.registry, clock=self.clock, capacity=4)
        with ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(lambda _: self.resolver.open(self.a, 'francestudent'), range(20)))
        successes = [item['application_handle'] for item in results if item['ok']]
        self.assertEqual(len(set(successes)), 4)
        sessions = [self.resolver.resolve(self.a, 'francestudent', h).internal_id for h in successes]
        self.assertEqual(len(set(sessions)), 4)
        self.assertEqual(len(self.resolver.entries), 4)
        with ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(lambda _: self.resolver.close_local(
                self.a, 'francestudent', successes[0]), range(20)))
        self.assertEqual(sum(item['ok'] for item in results), 1)
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', successes[0]))

    def test_identity_forgery_and_nonfinite_clock_fail_closed(self):
        forged = Identity(self.a.owner, '0' * 16, self.a.epoch)
        self.assertFalse(self.resolver.open(forged, 'francestudent')['ok'])
        handle = self.create()
        self.clock.value = -1
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', handle))
        self.clock.value = 0
        self.assertIsNotNone(self.resolver.resolve(self.a, 'francestudent', handle))
        self.clock.value = float('nan')
        self.assertIsNone(self.resolver.resolve(self.a, 'francestudent', handle))
        self.assertFalse(self.resolver.open(self.a, 'francestudent')['ok'])


if __name__ == '__main__':
    unittest.main()
