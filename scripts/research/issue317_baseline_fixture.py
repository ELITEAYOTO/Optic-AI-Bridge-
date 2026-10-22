"""Finite synthetic descendants. No network, private data, or persistent state."""
import os
import subprocess
import sys
import time

mode = sys.argv[1]
if mode == "hold":
    time.sleep(8)
elif mode in ("stdout", "stderr", "stdin", "timeout"):
    null = subprocess.DEVNULL
    subprocess.Popen([sys.executable, __file__, "hold"],
                     stdin=None if mode == "stdin" else null,
                     stdout=None if mode in ("stdout", "timeout") else null,
                     stderr=None if mode == "stderr" else null)
    if mode == "timeout":
        time.sleep(8)
elif mode == "output":
    os.write(1, b"x" * 100000)
elif mode == "both-output":
    os.write(2, b"e" * 12000)
    os.write(1, b"o" * 12000)
elif mode == "nonzero":
    sys.exit(7)
elif mode == "null-stdin":
    assert sys.stdin.read() == ""
elif mode != "normal":
    raise ValueError("unknown synthetic mode")
