"""One bounded prior-tool observation for generated alpha recovery, not authority."""
import asyncio
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import secrets

from diagnostics import CODE, TOOL, MAX_COUNTER, DIAGNOSTIC_TOOL
from project_resume import ordinary

MAX_BYTES = 1024
NAME = 'last-tool.json'
PENDING = '.last-tool.pending'
MUTATING = {'optic_fs_write', 'optic_fs_patch', 'optic_fs_mkdir', 'optic_task_start', 'optic_task_stop', 'optic_task_forget'}
KEYS = {'version', 'bridge_instance', 'sequence', 'tool', 'phase', 'ok', 'code',
        'outcome_uncertain', 'received_utc', 'finished_utc'}


def utc():
    return datetime.now(timezone.utc).isoformat(timespec='milliseconds')


def validate(value):
    if (type(value) is not dict or set(value) != KEYS
            or type(value['version']) is not int or value['version'] != 1
            or type(value['bridge_instance']) is not str
            or re.fullmatch('[0-9a-f]{16}', value['bridge_instance']) is None
            or type(value['sequence']) is not int or not 0 <= value['sequence'] <= MAX_COUNTER
            or type(value['tool']) is not str or not TOOL.fullmatch(value['tool'])
            or value['tool'] == DIAGNOSTIC_TOOL or value['phase'] not in ('admitted', 'completed')
            or type(value['outcome_uncertain']) is not bool):
        raise ValueError('project.checkpoint_invalid')
    if value['phase'] == 'admitted':
        if (value['ok'] is not None or value['code'] is not None or value['finished_utc'] is not None
                or value['outcome_uncertain'] != (value['tool'] in MUTATING)):
            raise ValueError('project.checkpoint_invalid')
    elif (type(value['ok']) is not bool
            or value['code'] is not None and (type(value['code']) is not str or not CODE.fullmatch(value['code']))
            or value['ok'] and (value['code'] is not None or value['outcome_uncertain'])
            or value['outcome_uncertain'] != (value['code'] in ('project.outcome_unknown', 'execution.outcome_unknown'))):
        raise ValueError('project.checkpoint_invalid')
    for key in ('received_utc', 'finished_utc'):
        item = value[key]
        if item is None and key == 'finished_utc' and value['phase'] == 'admitted':
            continue
        if type(item) is not str or len(item) > 40:
            raise ValueError('project.checkpoint_invalid')
        parsed = datetime.fromisoformat(item)
        if parsed.tzinfo is None or parsed.utcoffset().total_seconds() != 0:
            raise ValueError('project.checkpoint_invalid')
    return dict(value)


def pairs(items):
    result = {}
    for key, value in items:
        if key in result:
            raise ValueError('project.checkpoint_invalid')
        result[key] = value
    return result


class ToolCheckpoint:
    """Used only under the operator's existing exclusive generated-run lock.

    Two fixed paths, each <=1024bytes. Never stores inputs, source, paths, job IDs,
    credentials, native exception text or authorization. Not a durable task ledger.
    """
    def __init__(self, root):
        self.root = Path(root)
        ordinary(self.root, directory=True)
        self.instance = secrets.token_hex(8)
        self.sequence = -1
        self.current = None
        self.failed = False
        self._pending = None
        self.previous = self._previous()

    @property
    def busy(self):
        return self._pending is not None

    async def _background(self, operation, *arguments):
        # One retained off-loop I/O operation, no waiting queue. Caller cancellation
        # cannot release its slot before the filesystem operation really ends.
        if self._pending is not None:
            return None
        task = asyncio.create_task(asyncio.to_thread(operation, *arguments))
        self._pending = task
        def completed(done):
            self._pending = None
            if not done.cancelled():
                done.exception()
        task.add_done_callback(completed)
        return await asyncio.shield(task)

    async def begin_async(self, tool, sequence):
        return await self._background(self.begin, tool, sequence)

    async def finish_async(self, ticket, payload):
        return await self._background(self.finish, ticket, payload)

    def _read(self, path):
        ordinary(path)
        with path.open('rb') as stream:
            info = os.fstat(stream.fileno())
            if info.st_nlink != 1 or info.st_size > MAX_BYTES:
                raise ValueError('project.checkpoint_invalid')
            raw = stream.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError('project.checkpoint_invalid')
        return validate(json.loads(raw, object_pairs_hook=pairs))

    def _previous(self):
        pending = self.root / PENDING
        retained = self.root / NAME
        # A pending publication is newer than the last successfully replaced file.
        path = pending if pending.exists() or pending.is_symlink() else retained
        if not path.exists() and not path.is_symlink():
            return {'state': 'not_available', 'trusted_for_authorization': False}
        try:
            event = self._read(path)
            if path == pending:
                if retained.exists() or retained.is_symlink():
                    ordinary(retained)
                    if retained.stat().st_nlink != 1:
                        raise ValueError('project.checkpoint_invalid')
                # Preserve the newer observation across another immediate crash.
                # This promotes metadata only, never a process/effect receipt.
                os.replace(pending, retained)
            return {'state': 'observation_available', 'trusted_for_authorization': False,
                    'interrupted_publication': path == pending, 'event': event}
        except (OSError, ValueError, TypeError, RecursionError):
            self.failed = True
            return {'state': 'invalid', 'trusted_for_authorization': False}

    def _save(self, event):
        validate(event)
        raw = json.dumps(event, sort_keys=True, allow_nan=False).encode('ascii')
        if len(raw) > MAX_BYTES:
            raise ValueError('project.checkpoint_invalid')
        ordinary(self.root, directory=True)
        target = self.root / NAME
        if target.exists() or target.is_symlink():
            ordinary(target)
            if target.stat().st_nlink != 1:
                raise ValueError('project.checkpoint_invalid')
        pending = self.root / PENDING
        # Exclusive creation cannot follow a pre-existing sidecar or hard link.
        with pending.open('xb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(pending, target)

    def begin(self, tool, sequence):
        if self.failed:
            return None
        event = {'version': 1, 'bridge_instance': self.instance, 'sequence': sequence,
                 'tool': tool, 'phase': 'admitted', 'ok': None, 'code': None,
                 'outcome_uncertain': tool in MUTATING, 'received_utc': utc(), 'finished_utc': None}
        if type(sequence) is int and 0 <= sequence <= self.sequence:
            # SDK dispatch can reach us out of ingress order. Refuse this older
            # call before forwarding, without poisoning healthy persistence.
            return None
        try:
            self._save(event)
            self.current, self.sequence = event, sequence
            return sequence
        except (OSError, ValueError, TypeError):
            self.failed = True
            return None

    def finish(self, ticket, payload):
        # An older concurrent reply must not overwrite a newer admitted call.
        if ticket != self.sequence:
            return not self.failed
        if self.failed or self.current is None:
            return False
        code = payload.get('code') if payload.get('ok') is not True else None
        if code is not None and (type(code) is not str or not CODE.fullmatch(code)):
            code = 'project.unclassified_error'
        event = {**self.current, 'phase': 'completed', 'ok': payload.get('ok') is True,
                 'code': code, 'finished_utc': utc(),
                 'outcome_uncertain': code in ('project.outcome_unknown', 'execution.outcome_unknown')}
        try:
            self._save(event)
            self.current = event
            return True
        except (OSError, ValueError, TypeError):
            self.failed = True
            return False
