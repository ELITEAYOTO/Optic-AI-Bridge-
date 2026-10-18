import base64
import json
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from project_results import upstream as decode_upstream

from forward_core import ForwardError
from lifecycle_core import Registry, TOKENS
from node_profiles import configure_node
from project_gate import ProjectGate
from project_schema import normalize, SchemaError
from supervision_limits import ProjectFiles
from viewer_snapshot import FILES, write_viewer_ready


class FileBudget(unittest.IsolatedAsyncioTestCase):
    def test_internal_snapshot_envelope_is_selected_and_bounded(self):
        raw = b'x' * 32769
        response = SimpleNamespace(is_error=False, structured_content={
            'data': base64.b64encode(raw).decode(), 'eof': False}, content=[])
        with self.assertRaisesRegex(ForwardError, 'link.output_bound'):
            decode_upstream(response)
        self.assertEqual(decode_upstream(response, 32768)['data'], response.structured_content['data'])
        response.structured_content['data'] += 'x' * 2048
        with self.assertRaisesRegex(ForwardError, 'link.output_bound'):
            decode_upstream(response, 32768)

    def test_operator_limit_counts_utf8_and_cannot_be_selected_remotely(self):
        arguments = {'path': 'src/large.js', 'content': 'é' * 16384,
                     'expected_kind': 'absent'}
        with self.assertRaises(SchemaError):
            normalize('optic_fs_write', arguments)
        _, selected = normalize('optic_fs_write', arguments, max_file_bytes=32768)
        self.assertEqual(len(base64.b64decode(selected['content_base64'])), 32768)
        for value in ('é' * 16385, '\ud800'):
            with self.assertRaises(SchemaError):
                normalize('optic_fs_write', {**arguments, 'content': value}, max_file_bytes=32768)
        with self.assertRaises(SchemaError):
            normalize('optic_fs_write', {**arguments, 'max_file_bytes': 32768})
        for invalid in (True, 0, 8193, 65536, '32768'):
            with self.assertRaises(ValueError):
                normalize('optic_project_info', {}, max_file_bytes=invalid)

    async def test_large_file_pages_stay_small_and_patch_growth_is_refused(self):
        state = Registry()
        owner = state.authenticate(b'Bearer ' + TOKENS['A'][0])
        raw = b'x' * 32768
        calls = []
        async def upstream(name, arguments):
            calls.append((name, arguments))
            if name == 'fs_read':
                return {'path': arguments['path'], 'data': base64.b64encode(raw).decode(),
                        'eof': True, 'content_version': 'a' * 64}
            return {'content_version': 'b' * 64}
        gate = ProjectGate(state, owner, upstream, read_page_bytes=4096, max_file_bytes=32768)
        page = await gate.call(owner, 'optic_fs_read', {'path': 'src/a.js', 'offset': 12288})
        self.assertEqual(page['file_bytes'], 32768)
        self.assertEqual(page['next_offset'], 16384)
        self.assertEqual(len(page['text']), 4096)
        self.assertFalse(page['eof'])
        self.assertEqual(calls[0][1]['max_bytes'], 32769)
        patch = {'path': 'src/a.js', 'offset': 32768, 'remove_bytes': 0,
                 'insert': '!', 'expected_version': 'a' * 64}
        with self.assertRaisesRegex(ForwardError, '^project.patch_limit$'):
            await gate.call(owner, 'optic_fs_patch', patch)
        self.assertFalse(any(name == 'fs_apply_patch' for name, _ in calls))
        await gate.call(owner, 'optic_fs_patch', {**patch, 'offset': 32767, 'remove_bytes': 1})
        self.assertEqual(calls[-1][0], 'fs_apply_patch')
        before = len(calls)
        default = ProjectGate(state, owner, upstream)
        with self.assertRaises(ForwardError):
            await default.call(owner, 'optic_fs_read', {'path': 'src/a.js', 'offset': 12288})
        self.assertEqual(len(calls), before)

    def test_inventory_and_node_resume_use_the_selected_limit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / 'project'
            (project / 'src').mkdir(parents=True)
            node = root / 'node.exe'
            node.write_bytes(b'public placeholder, never executed')
            configure_node(project, node)
            source = project / 'src/model.js'
            raw = b'// public source\n' * 900
            source.write_bytes(raw)
            with self.assertRaises(ValueError):
                ProjectFiles.inspect(project)
            self.assertIn('src/model.js', ProjectFiles.inspect(project, max_file_bytes=32768).paths)
            with self.assertRaisesRegex(ValueError, 'resume_source_bound'):
                configure_node(project, node, resume=True)
            profiles, _ = configure_node(project, node, resume=True, max_file_bytes=32768)
            self.assertEqual(set(profiles), {'check', 'test'})
            self.assertEqual(source.read_bytes(), raw)
            source.write_bytes(b'x' * 32769)
            with self.assertRaises(ValueError):
                ProjectFiles.inspect(project, max_file_bytes=32768)

    def test_large_viewer_snapshot_records_its_budget_without_changing_legacy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'project/src'
            source.mkdir(parents=True)
            for name in FILES:
                (source / name).write_bytes(b'x' * 12000)
            with self.assertRaises(ValueError):
                write_viewer_ready(root)
            self.assertFalse((root / 'viewer-ready.json').exists())
            write_viewer_ready(root, max_file_bytes=32768)
            marker = json.loads((root / 'viewer-ready.json').read_bytes())
            self.assertEqual(marker['version'], 2)
            self.assertEqual(marker['max_file_bytes'], 32768)
            self.assertEqual(set(marker['files']), set(FILES))
