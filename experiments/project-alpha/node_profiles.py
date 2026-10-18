"""Locally selected Node and fixed generated source slots; no remote configuration."""
from pathlib import Path
from project_schema import MAX_CONTENT, file_limit

SOURCE_FILES = ('src/model.js', 'src/app.js', 'src/tests.js')


def configure_node(project, executable, resume=False, max_file_bytes=MAX_CONTENT):
    maximum = file_limit(max_file_bytes)
    node = Path(executable).resolve(strict=True)
    if not node.is_file() or node.name.lower() != 'node.exe':
        raise ValueError('project.node_executable_required')
    root = Path(project).resolve(strict=True)
    # These are newly generated files only. Never overwrite an existing source.
    initial = (
        'module.exports.add = (a, b) => a + b;\n',
        '// Browser application entry.\n',
        "const assert = require('node:assert/strict');\n"
        "const model = require('./model.js');\n"
        "assert.equal(model.add(2, 3), 5);\nconsole.log('ALPHA_TEST_PASS');\n",
    )
    if resume:
        from project_resume import ordinary
        for relative in SOURCE_FILES:
            path = root / relative
            ordinary(path)
            with path.open('rb') as stream:
                raw = stream.read(maximum + 1)
            if not 1 <= len(raw) <= maximum:
                raise ValueError('project.resume_source_bound')
            raw.decode('utf-8', errors='strict')
    else:
        for relative, content in zip(SOURCE_FILES, initial):
            with (root / relative).open('x', encoding='utf-8', newline='\n') as stream:
                stream.write(content)
    def profile(arguments):
        return {'executable': str(node), 'args': arguments, 'cwd': None,
                'env_allowlist': [], 'network': False, 'timeout_ms': 10000,
                'output_budget': 65536, 'memory_bytes': 268435456, 'process_count': 1}
    paths = [str(root / relative) for relative in SOURCE_FILES]
    check = "const fs=require('node:fs'),vm=require('node:vm');for(const p of process.argv.slice(1)){new vm.Script(fs.readFileSync(p,'utf8'),{filename:p});}console.log('ALPHA_CHECK_PASS');"
    profiles = {'check': profile(['-e', check, *paths]),
                'test': profile(['--preserve-symlinks-main', '--preserve-symlinks', paths[2]])}
    arguments = ['--allow-executable', 'interpreter:' + str(node), '--allow-isolated-node', str(node)]
    for relative in SOURCE_FILES:
        arguments.extend(['--allow-process-read-file', str(node), relative])
    return profiles, arguments
