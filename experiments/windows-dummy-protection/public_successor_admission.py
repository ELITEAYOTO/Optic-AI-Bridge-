"""One-process, no-queue public fixture coordination. No authentication authority."""
import threading

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

