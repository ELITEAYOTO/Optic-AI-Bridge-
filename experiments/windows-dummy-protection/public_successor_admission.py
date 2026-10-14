"""One-process, no-queue public fixture coordination. No authentication authority."""
import threading
import hashlib

from create_only_successor import BASELINE, RECORDS, SuccessorError
from public_successor_reader import observe_public_successor
from published_public_successor import PublishedPublicSuccessor


class PublicSuccessorAdmission:
    """Only this object's completed publication may become locally confirmed.

    The lock covers observation and the entire synchronous publication. Busy
    callers fail immediately. It is not a process lock or an I/O deadline.
    Existing data is never adopted, including a validated pair after restart.
    Returned snapshots grant no lease, authorization or exactly-once effects.
    """
    def __init__(self, directory):
        self._directory = directory
        self._lock = threading.Lock()
        self._state = 'blocked'
        self._operation = None
        self._probes = 0
        try:
            if observe_public_successor(directory)['status'] == 'empty':
                self._state = 'empty'
        except Exception:
            pass

    def _enter(self):
        if not self._lock.acquire(blocking=False):
            raise SuccessorError('successor_admission.busy')

    def _snapshot(self):
        return {'status': self._state, 'operation': self._operation,
                'fixture_only': True, 'authority': 'none'}

    def _observe_confirmed(self):
        if self._state != 'confirmed':
            raise SuccessorError('successor_admission.unconfirmed')
        observed = observe_public_successor(self._directory)
        if observed['status'] != 'validated' or observed['operation'] != self._operation:
            raise SuccessorError('successor_admission.changed')

    def _measure_public(self):
        # Fixed public in-memory computation only, never a caller callback.
        return hashlib.sha256(RECORDS[self._operation]).hexdigest()

    def probe_confirmed(self):
        """Keep ownership through fixed public work and final observation.

        At most 32 attempts per object; no effects, lease or durable results.
        External writers remain outside the lock. Post-check failure suppresses
        the result but cannot undo computation that has already happened.
        """
        self._enter()
        try:
            if self._state != 'confirmed' or self._probes >= 32:
                raise SuccessorError('successor_admission.unavailable')
            self._probes += 1
            try:
                self._observe_confirmed()
                digest = self._measure_public()
                self._observe_confirmed()
                return {**self._snapshot(), 'public_digest': digest,
                        'attempt': self._probes}
            except BaseException:
                self._state, self._operation = 'blocked', None
                raise SuccessorError('successor_admission.blocked') from None
        finally:
            self._lock.release()

    def close(self):
        """Local bookkeeping only. Busy refusal is NOT a completed close."""
        self._enter()
        try:
            self._state, self._operation = 'blocked', None
            return {'closed': True, 'fixture_only': True, 'authority': 'none'}
        finally:
            self._lock.release()

    def read(self):
        self._enter()
        try:
            if self._state == 'blocked':
                raise SuccessorError('successor_admission.blocked')
            observed = observe_public_successor(self._directory)
            expected = 'validated' if self._state == 'confirmed' else 'empty'
            if observed['status'] != expected or observed['operation'] != self._operation:
                raise SuccessorError('successor_admission.changed')
            return self._snapshot()
        except BaseException:
            self._state, self._operation = 'blocked', None
            raise SuccessorError('successor_admission.blocked') from None
        finally:
            self._lock.release()

    def publish_once(self, operation, expected):
        # Invalid caller arguments do not consume the single transition.
        if type(operation) is not str or operation not in RECORDS or type(expected) is not bytes or expected != BASELINE:
            raise SuccessorError('successor_admission.invalid_input')
        self._enter()
        if self._state != 'empty':
            self._lock.release()
            raise SuccessorError('successor_admission.unavailable')
        try:
            # Mark uncertain BEFORE the first publisher I/O. Failure cannot
            # restore admission merely because public bytes later validate.
            self._state = 'blocked'
            result = PublishedPublicSuccessor(self._directory, operation).claim_once(expected)
            observed = observe_public_successor(self._directory)
            if result != observed or observed['status'] != 'validated' or observed['operation'] != operation:
                raise SuccessorError('successor_admission.changed')
            self._state, self._operation = 'confirmed', operation
            return self._snapshot()
        except BaseException:
            self._state, self._operation = 'blocked', None
            raise SuccessorError('successor_admission.blocked') from None
        finally:
            self._lock.release()

