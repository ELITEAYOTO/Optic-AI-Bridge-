"""Explicit local selection and exclusive ownership of retained alpha runs."""
from contextlib import contextmanager
from pathlib import Path
import os
import re
import stat
import json


def previous_session(root, fresh):
    report = root / 'last-stop.json'
    if not report.exists():
        return {'state': 'no_previous_session' if fresh else 'previous_shutdown_unconfirmed'}
    try:
        ordinary(report)
        with report.open('rb') as stream:
            raw = stream.read(1025)
        if len(raw) > 1024:
            raise ValueError()
        value = json.loads(raw)
        if (type(value) is not dict or type(value.get('version')) is not int or value['version'] != 1
                or value.get('reason') not in ('operator', 'expiry', 'service_exit', 'revoked')
                or type(value.get('revocation_confirmed')) is not bool
                or type(value.get('work_drained')) is not bool):
            raise ValueError()
        return {'state': 'previous_stop_report_available',
                **{key: value[key] for key in ('reason', 'revocation_confirmed', 'work_drained')}}
    except (OSError, ValueError, TypeError, RecursionError):
        return {'state': 'previous_stop_report_invalid'}


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
        fresh = True
        stream.write(b'0')
        stream.flush()
    except FileExistsError:
        fresh = False
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
        previous = previous_session(root, fresh)
        # An earlier viewer snapshot must not authorize an active editing run.
        for name in ('viewer-ready.json', 'last-stop.json'):
            ready = root / name
            if ready.exists():
                ordinary(ready)
                ready.unlink()
        yield previous
    finally:
        if locked:
            if os.name == 'nt':
                stream.seek(0)
                msvcrt.locking(stream.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(stream.fileno(), fcntl.LOCK_UN)
        stream.close()
