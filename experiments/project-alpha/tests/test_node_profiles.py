import tempfile
from pathlib import Path
import unittest

from execution_gate import freeze_profiles
from node_profiles import configure_node, SOURCE_FILES


class NodeProfiles(unittest.TestCase):
    def test_generated_slots_fixed_tasks_and_no_source_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / 'project'
            (project / 'src').mkdir(parents=True)
            node = root / 'node.exe'
            node.write_bytes(b'public test placeholder, never executed')
            profiles, arguments = configure_node(project, node)
            self.assertEqual(set(freeze_profiles(profiles)), {'check', 'test'})
            self.assertEqual(arguments.count('--allow-process-read-file'), 3)
            self.assertTrue(all(profile['network'] is False and profile['process_count'] == 1
                                for profile in profiles.values()))
            self.assertEqual([path for path in SOURCE_FILES if (project / path).is_file()], list(SOURCE_FILES))
            original = (project / SOURCE_FILES[0]).read_bytes()
            with self.assertRaises(FileExistsError):
                configure_node(project, node)
            self.assertEqual((project / SOURCE_FILES[0]).read_bytes(), original)

    def test_wrong_executable_rejected_before_any_sources_created(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'src').mkdir()
            for name in ('cmd.exe', 'electron.exe'):
                wrong = root / name
                wrong.write_bytes(b'public placeholder')
                with self.assertRaisesRegex(ValueError, 'node_executable_required'):
                    configure_node(root, wrong)
                self.assertEqual(list((root / 'src').iterdir()), [])
