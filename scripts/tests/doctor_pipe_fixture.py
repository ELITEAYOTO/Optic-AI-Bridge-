"""Synthetic Windows pipe fixtures only; every sleeper has an 8-second ceiling."""
import os
from pathlib import Path
import subprocess
import sys
import time

mode, ready = sys.argv[1:3]
if mode == "phase-holder":
    Path(ready).write_text("synthetic", encoding="ascii")
    payload = b"x" * 5000 + b"\n"
    payload += b"OPTIC_STARTUP phase=unknown-private-label elapsed_ms=1\n"
    payload += b"".join(("OPTIC_STARTUP phase=entry elapsed_ms=%d\n" % i).encode("ascii") for i in range(30))
    payload += b"OPTIC_STARTUP phase=transport-created elapsed_ms=42\n"
    os.write(2, payload)
    time.sleep(8)
    sys.exit(0)
if mode == "holder":
    Path(ready).write_text("synthetic", encoding="ascii")
    time.sleep(8)
    sys.exit(0)
if mode in ["eof-descendant", "timeout-descendant", "phase-flood-parent"]:
    subprocess.Popen([sys.executable, "-I", __file__, "phase-holder" if mode == "phase-flood-parent" else "holder", ready],
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                     stderr=None, close_fds=True)
    until = time.monotonic() + 2
    while not Path(ready).exists():
        assert time.monotonic() < until
        time.sleep(0.005)
    if mode == "eof-descendant":
        sys.exit(0)
    time.sleep(8)
elif mode == "normal":
    os.write(1, b'{"jsonrpc":"2.0","id":1,"result":{"ok":true}}\n')
    sys.exit(0)
elif mode == "eof-live":
    os.close(1)
    os.write(2, b"OPTIC_STARTUP phase=entry elapsed_ms=0\n")
    Path(ready).write_text("synthetic", encoding="ascii")
    time.sleep(8)
elif mode == "partial-utf8":
    os.close(1)
    os.write(2, b"\xe2")
    Path(ready).write_text("synthetic", encoding="ascii")
    time.sleep(8)
elif mode == "oversized":
    os.close(1)
    Path(ready).write_text("synthetic", encoding="ascii")
    os.write(2, b"x" * 40000)
    time.sleep(8)
elif mode == "capture-live":
    os.close(1)
    Path(ready).write_text("synthetic", encoding="ascii")
    time.sleep(8)
else:
    raise RuntimeError("unknown fixture")
