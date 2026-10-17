import asyncio
import unittest

from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from project_gate import ProjectGate
from project_schema import normalize, SchemaError, DIRECTORY_TOOL
from supervision_limits import ProjectFiles, MAX_DIRECTORIES


class Directories(unittest.IsolatedAsyncioTestCase):
    async def test_explicit_grant_quotas_conflicts_and_uncertain_caller_are_retained(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        calls, completed = [], asyncio.Event()
        async def upstream(name, args):
            calls.append((name, args))
            await completed.wait()
            return {'path': args['path'], 'created': True}
        args = {'path': 'src/components', 'expected_kind': 'absent'}
        gate = ProjectGate(state, owner, upstream)
        with self.assertRaisesRegex(ForwardError, 'project.directory_authority_unavailable'):
            await gate.call(owner, DIRECTORY_TOOL, args)
        self.assertFalse(calls)
        budget = ProjectFiles(directories=('src',))
        gate = ProjectGate(state, owner, upstream, deadline=.01, file_budget=budget, directory_create=True)
        with self.assertRaisesRegex(ForwardError, 'project.outcome_unknown'):
            await gate.call(owner, DIRECTORY_TOOL, args)
        with self.assertRaisesRegex(ForwardError, 'link.busy'):
            await gate.call(owner, DIRECTORY_TOOL, args)
        self.assertEqual(len(calls), 1)
        completed.set()
        self.assertTrue(await gate.stop())
        self.assertEqual(budget.directories, {'src', 'src/components'})
        budget = ProjectFiles(directories=(f'src/d{i}' for i in range(MAX_DIRECTORIES)))
        gate = ProjectGate(state, owner, upstream, file_budget=budget, directory_create=True)
        with self.assertRaisesRegex(ForwardError, 'project.directory_capacity'):
            await gate.call(owner, DIRECTORY_TOOL, args)
        self.assertEqual(len(calls), 1)

    def test_mkdir_schema_requires_absence_and_no_hidden_recursive_option(self):
        expected = ('fs_create_directory', {'path': 'src/components', 'expected': 'absent'})
        self.assertEqual(normalize(DIRECTORY_TOOL, {'path': 'src/components', 'expected_kind': 'absent'}), expected)
        for args in ({'path': 'src/a'}, {'path': 'src/a', 'expected_kind': 'present'},
                     {'path': 'src/a', 'expected_kind': 'absent', 'parents': True},
                     {'path': 'src/../escape', 'expected_kind': 'absent'}):
            with self.assertRaises(SchemaError):
                normalize(DIRECTORY_TOOL, args)
