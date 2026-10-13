import asyncio
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'credential-lifecycle'))
sys.path.insert(0, str(ROOT / 'francestudent-probe'))
from lifecycle_core import Registry, TOKENS
from group_core import Groups


class Tests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.registry, self.now = Registry(), 0
        self.a = self.registry.authenticate(b'Bearer ' + TOKENS['A'][0])
        self.b = self.registry.authenticate(b'Bearer ' + TOKENS['B'][0])
        self.groups = Groups(self.registry, clock=lambda: self.now)

    def create(self, owner=None):
        return self.groups.create(owner or self.a)['application_handle']

    async def pair(self, handles):
        ready, release = asyncio.Event(), asyncio.Event()
        count = 0
        async def sleep(_):
            nonlocal count
            count += 1
            if count == 2:
                ready.set()
            await release.wait()
        self.groups.sleeper = sleep
        tasks = [asyncio.create_task(self.groups.wait(self.a, handle, index + 1)) for index, handle in enumerate(handles)]
        try:
            await asyncio.wait_for(ready.wait(), 1)
            self.now += 3
        finally:
            release.set()
        return await asyncio.gather(*tasks)

    async def test_same_group_overlaps_and_records_are_snapshots(self):
        handle = self.create()
        results = await self.pair([handle, handle])
        self.assertTrue(all(item['ok'] and item['overlap_observed'] for item in results))
        self.assertEqual(sorted(item['active_on_entry'] for item in results), [1, 2])
        report = self.groups.report(self.a, handle)
        self.assertTrue(report['completed_pair_overlap'])
        report['records'][0]['state'] = 'fake'
        self.assertEqual(self.groups.report(self.a, handle)['records'][0]['state'], 'completed')
        self.assertEqual(self.groups.active, 0)

    async def test_other_group_cannot_create_false_overlap(self):
        first, second = self.create(), self.create()
        results = await self.pair([first, second])
        self.assertTrue(all(item['ok'] and not item['overlap_observed'] for item in results))
        self.assertFalse(self.groups.report(self.a, first)['completed_pair_overlap'])

    async def test_sequential_intervals_do_not_overlap(self):
        async def sleep(_):
            self.now += 3
        self.groups.sleeper = sleep
        handle = self.create()
        results = [await self.groups.wait(self.a, handle) for _ in range(2)]
        self.assertTrue(all(not item['overlap_observed'] for item in results))
        self.assertFalse(self.groups.report(self.a, handle)['completed_pair_overlap'])

    async def test_cancel_cleanup_and_record_budget(self):
        ready = asyncio.Event()
        async def sleep(_):
            ready.set()
            await asyncio.Event().wait()
        self.groups.sleeper = sleep
        handle = self.create()
        pending = asyncio.create_task(self.groups.wait(self.a, handle))
        await asyncio.wait_for(ready.wait(), 1)
        pending.cancel()
        with self.assertRaises(asyncio.CancelledError):
            await pending
        self.assertEqual(self.groups.active, 0)
        self.assertEqual(self.groups.report(self.a, handle)['records'][0]['state'], 'cancelled')
        async def immediate(_):
            self.now += 1
        self.groups.sleeper = immediate
        for _ in range(7):
            self.assertTrue((await self.groups.wait(self.a, handle))['ok'])
        self.assertEqual((await self.groups.wait(self.a, handle))['code'], 'probe.group_admission')
        self.assertEqual(len(self.groups.report(self.a, handle)['records']), 8)

    async def test_owner_epoch_expiry_and_malformed_fail_closed(self):
        handle = self.create()
        self.assertFalse(self.groups.report(self.b, handle)['ok'])
        self.assertFalse((await self.groups.wait(self.a, 'x' * 100000))['ok'])
        self.registry.control('rotate', 'A')
        new = self.registry.authenticate(b'Bearer ' + TOKENS['A'][1])
        self.assertFalse(self.groups.report(new, handle)['ok'])
        self.assertTrue(self.groups.create(self.b)['ok'])
        handle = self.groups.create(new)['application_handle']
        self.now = 120
        self.assertFalse(self.groups.report(new, handle)['ok'])

    async def test_capacity_budget_and_clock_bounds(self):
        self.groups = Groups(self.registry, clock=lambda: self.now, capacity=1, budget=2)
        self.create()
        self.assertFalse(self.groups.create(self.a)['ok'])
        self.now = 120
        self.create()
        self.now = 240
        self.assertFalse(self.groups.create(self.a)['ok'])
        self.assertEqual(len(self.groups.issued), 2)
        self.now = float('nan')
        self.assertFalse(self.groups.create(self.a)['ok'])

    async def test_revoked_while_waiting_denies_result_and_frees_admission(self):
        ready, release = asyncio.Event(), asyncio.Event()
        async def sleep(_):
            ready.set()
            await release.wait()
        self.groups.sleeper = sleep
        handle = self.create()
        pending = asyncio.create_task(self.groups.wait(self.a, handle))
        await asyncio.wait_for(ready.wait(), 1)
        self.registry.control('revoke', 'A')
        release.set()
        self.assertFalse((await pending)['ok'])
        self.assertEqual(self.groups.active, 0)

    async def test_global_and_per_group_pending_bounds(self):
        count = 0
        ready, release = asyncio.Event(), asyncio.Event()
        async def sleep(_):
            nonlocal count
            count += 1
            if count == 8:
                ready.set()
            await release.wait()
        self.groups.sleeper = sleep
        first, second, third = self.create(), self.create(), self.create()
        tasks = [asyncio.create_task(self.groups.wait(self.a, h)) for h in [first] * 4 + [second] * 4]
        try:
            await asyncio.wait_for(ready.wait(), 1)
            self.assertEqual(self.groups.active, 8)
            self.assertFalse((await self.groups.wait(self.a, first))['ok'])
            self.assertFalse((await self.groups.wait(self.a, third))['ok'])
            self.assertEqual(self.groups.report(self.a, third)['records'], [])
            self.now = 3
        finally:
            release.set()
            await asyncio.gather(*tasks)
        self.assertEqual(self.groups.active, 0)
