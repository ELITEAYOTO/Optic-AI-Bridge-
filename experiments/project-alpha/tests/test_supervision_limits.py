import math
from pathlib import Path
import tempfile
import unittest

from supervision_limits import WindowBudget, ProjectFiles, MAX_FILES


class Limits(unittest.TestCase):
    def test_window_budget_refuses_bursts_and_recovers_only_after_window(self):
        budget = WindowBudget(2, 60, 0)
        self.assertTrue(budget.take(0))
        self.assertTrue(budget.take(1))
        self.assertFalse(budget.take(59.9))
        self.assertTrue(budget.take(60))
        self.assertEqual(budget.used, 1)
        self.assertFalse(budget.take(59))
        self.assertFalse(budget.take(120))
        self.assertTrue(budget.failed)
        for invalid in (math.nan, math.inf, True):
            with self.assertRaises(ValueError):
                WindowBudget(2, 60, invalid)

    def test_file_slots_are_conservative_bounded_and_case_insensitive(self):
        budget = ProjectFiles()
        for index in range(MAX_FILES):
            budget.reserve(f'src/file{index}.js')
        budget.reserve('src/FILE0.js')
        self.assertEqual(len(budget.paths), MAX_FILES)
        with self.assertRaisesRegex(ValueError, 'project.file_capacity'):
            budget.reserve('src/extra.js')

    def test_startup_inventory_is_bounded_and_refuses_oversized_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'src').mkdir()
            (root / 'src/a.js').write_bytes(b'x' * 8192)
            self.assertEqual(ProjectFiles.inspect(root).paths, {'src/a.js'})
            (root / 'src/a.js').write_bytes(b'x' * 8193)
            with self.assertRaisesRegex(ValueError, 'project.inventory_invalid'):
                ProjectFiles.inspect(root)
            (root / 'src/a.js').unlink()
            for index in range(MAX_FILES + 1):
                (root / f'src/f{index}').touch()
            with self.assertRaisesRegex(ValueError, 'project.file_capacity'):
                ProjectFiles.inspect(root)


class FileAdmission(unittest.IsolatedAsyncioTestCase):
    async def test_quota_denies_creation_before_dispatch_but_keeps_reads_available(self):
        from lifecycle_core import Registry, TOKENS
        from project_gate import ProjectGate
        from forward_core import ForwardError
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        calls = []
        async def upstream(name, args):
            calls.append(name)
            return {'capabilities': []}
        budget = ProjectFiles(f'src/f{i}' for i in range(MAX_FILES))
        gate = ProjectGate(state, owner, upstream, file_budget=budget)
        with self.assertRaisesRegex(ForwardError, 'project.file_capacity'):
            await gate.call(owner, 'optic_fs_write', {'path': 'src/new.js',
                'content': 'public', 'expected_kind': 'absent'})
        self.assertFalse(calls)
        await gate.call(owner, 'optic_project_info', {})
        self.assertEqual(calls, ['session_info'])
