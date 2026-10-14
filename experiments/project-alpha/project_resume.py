"""Explicit local selection and exclusive ownership of retained alpha runs."""
from contextlib import contextmanager
from pathlib import Path
import os
import re
import stat


def ordinary(path, directory=False):
    info = path.lstat()
    if (stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400
            or not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode))):
        raise ValueError('project.resume_invalid_object')


def select_run(output_root, name):
    if not isinstance(name, str) or re.fullmatch(r'alpha-[a-z0-9_-]{1,64}', name) is None:
        raise ValueError('project.resume_invalid_name')
    output = Path(output_root).resolve(strict=True)
    root = output / name
    for directory in (root, root / 'project', root / 'project/src', root / 'journal'):
        ordinary(directory, directory=True)
    # No caller-selected absolute project, executable, or permission configuration.
    return root, root / 'project', root / 'journal'


@contextmanager
def session_lock(root):
    path = root / '.operator-session.lock'
    try:
        stream = path.open('x+b')
        stream.write(b'0')
        stream.flush()
    except FileExistsError:
        ordinary(path)
        stream = path.open('r+b')
    locked = False
    try:
        if os.name == 'nt':
            import msvcrt
            stream.seek(0)
            msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        locked = True
        # An earlier viewer snapshot must not authorize an active editing run.
        ready = root / 'viewer-ready.json'
        if ready.exists():
            ordinary(ready)
            ready.unlink()
        yield
    finally:
        if locked:
            if os.name == 'nt':
                stream.seek(0)
                msvcrt.locking(stream.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(stream.fileno(), fcntl.LOCK_UN)
        stream.close()
