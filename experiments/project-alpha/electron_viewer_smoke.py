"""Owned hidden Electron viewer probe, not MCP GUI-process authorization."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from viewer_snapshot import write_viewer_ready


def run(executable, max_file_bytes=8192):
    from project_schema import file_limit
    file_limit(max_file_bytes)
    with tempfile.TemporaryDirectory(prefix='optic-electron-viewer-') as directory:
        output = Path(directory)
        root = output / 'alpha-public-smoke'
        source = root / 'project' / 'src'
        source.mkdir(parents=True)
        contents = {'index.html': '<!doctype html><html><head><link rel="stylesheet" href="style.css"></head><body><h1>Public viewer probe</h1><script src="model.js"></script><script src="app.js"></script></body></html>',
                    'style.css': 'body{font-family:system-ui}',
                    'app.js': 'document.body.dataset.publicProbe="yes";',
                    'model.js': 'globalThis.PublicModel={value:1};', 'tests.js': '// public probe only\n'}
        if max_file_bytes == 32768:
            contents['style.css'] += '/*' + 'x' * 12000 + '*/'
        for name, text in contents.items():
            (source / name).write_text(text, encoding='utf-8')
        write_viewer_ready(root, max_file_bytes=max_file_bytes)
        main = Path(__file__).parent / 'viewer' / 'electron_main.cjs'
        for mode in ('--smoke', '--smoke-repeat'):
            result = subprocess.run([str(Path(executable).resolve(strict=True)), str(main), str(root), str(output), mode],
                                    capture_output=True, text=True, timeout=35)
            assert result.returncode == 0, {'exit': result.returncode, 'stdout': result.stdout[-2048:], 'stderr': result.stderr[-2048:]}
            assert 'ELECTRON_VIEWER_SMOKE_PASS' in result.stdout, result.stdout[-2048:]
        exports = list((root / 'exports').glob('*.json'))
        assert len(exports) == 2 and all(json.loads(file.read_text()) == {'public': True} for file in exports)
    print('ELECTRON VIEWER PASS: hidden native window, Node/process absent in renderer, external network/file/navigation denied, localStorage survives reopen and bounded fixed-directory JSON export passed. Operator viewer only; no MCP GUI process grant.')


if __name__ == '__main__':
    run(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 8192)
