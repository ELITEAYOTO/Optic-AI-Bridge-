"""Real interpreter-exit regression; never pipe-block the test process itself."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class ProofShutdown(unittest.TestCase):
    def child(self, code, blocked=False):
        env = dict(os.environ, PYTHONPATH=str(ROOT))
        with tempfile.TemporaryFile() as errors:
            child = subprocess.Popen([sys.executable, '-c', code], env=env,
                                     stdout=subprocess.PIPE, stderr=errors)
            try:
                if blocked:
                    code = child.wait(timeout=8)
                    output = b''
                else:
                    output, _ = child.communicate(timeout=8)
                    code = child.returncode
                errors.seek(0)
                error = errors.read(8192)
                return code, output, error
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=3)
                child.stdout.close()

    def test_full_pipe_default_sink_exits_without_buffered_writer_fatal(self):
        code, _, error = self.child('''import time
from proof_channel import ProofChannel
c=ProofChannel()
for _ in range(128): c.emit('x'*4096)
time.sleep(.15)
assert not c.close(), 'test requires a blocked sink'
''', blocked=True)
        self.assertEqual(code, 0, error)
        self.assertNotIn(b'_enter_buffered_busy', error)

    def test_normal_native_sink_drains_exact_records_and_repeated_close(self):
        code, output, error = self.child('''from proof_channel import ProofChannel
c=ProofChannel()
assert c.emit('first')
assert c.emit('last')
assert c.close()
assert c.close()
assert c.snapshot()['delivered']==2
''')
        self.assertEqual(code, 0, error)
        self.assertEqual(output, b'first\nlast\n')

    def test_broken_pipe_closes_admission_without_echoing_exception(self):
        code, output, error = self.child('''import os,time
from proof_channel import ProofChannel
read,write=os.pipe()
os.close(read)
c=ProofChannel()
os.close(c.output_fd)
c.output_fd=write
assert c.emit('public')
c.worker.join(timeout=2)
assert not c.worker.is_alive()
assert c.snapshot()['failed']
assert not c.emit('late')
assert c.close()
''')
        self.assertEqual(code, 0, error)
        self.assertEqual(output, b'')
        self.assertEqual(error, b'')
