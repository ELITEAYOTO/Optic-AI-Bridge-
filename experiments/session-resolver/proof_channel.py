"""Bounded best-effort proof output; sink I/O never runs in producer threads."""
import os
import queue
import sys
import threading

_DEFAULT_SINK = object()


class ProofChannel:
    def __init__(self, sink=_DEFAULT_SINK, capacity=128, budget=4096):
        if ((sink is not _DEFAULT_SINK and not callable(sink)) or type(capacity) is not int or not 1 <= capacity <= 128
                or type(budget) is not int or not 1 <= budget <= 4096):
            raise ValueError('invalid fixed proof limits')
        # A daemon must never hold Python's buffered stdout lock at interpreter
        # shutdown. Duplicate only the native descriptor; the worker owns it.
        # A full pipe can still stall this best-effort sink, but cannot stall
        # producers or leave Python's BufferedWriter locked during finalization.
        self.output_fd = os.dup(sys.stdout.fileno()) if sink is _DEFAULT_SINK else None
        self.sink = self._write_native if sink is _DEFAULT_SINK else sink
        self.budget = budget
        self.queue = queue.Queue(maxsize=capacity)
        self.lock = threading.Lock()
        self.accepted = self.delivered = self.dropped = 0
        self.closed = self.failed = False
        self.worker = threading.Thread(target=self._run, daemon=True)
        try:
            self.worker.start()
        except BaseException:
            if self.output_fd is not None:
                os.close(self.output_fd)
            raise

    def _write_native(self, line):
        pending = (line + '\n').encode('ascii')
        while pending:
            written = os.write(self.output_fd, pending)
            if written <= 0:
                raise OSError('proof output unavailable')
            pending = pending[written:]

    def emit(self, line):
        with self.lock:
            if (self.closed or self.failed or self.accepted >= self.budget
                    or not isinstance(line, str) or not line.isascii() or len(line) > 4096):
                self.dropped = min(4096, self.dropped + 1)
                return False
            try:
                self.queue.put_nowait(line)
            except queue.Full:
                self.dropped = min(4096, self.dropped + 1)
                return False
            self.accepted += 1
            return True

    def _run(self):
        try:
            self._deliver()
        finally:
            if self.output_fd is not None:
                os.close(self.output_fd)

    def _deliver(self):
        while True:
            with self.lock:
                if self.closed and self.queue.empty():
                    return
            try:
                line = self.queue.get(timeout=.05)
            except queue.Empty:
                continue
            try:
                # No producer/registry lock held while stdout might stall.
                self.sink(line)
            except Exception:
                with self.lock:
                    self.failed = self.closed = True
                return
            finally:
                self.queue.task_done()
            with self.lock:
                self.delivered += 1

    def snapshot(self):
        with self.lock:
            return {'accepted': self.accepted, 'delivered': self.delivered,
                    'dropped': self.dropped, 'queued': self.queue.qsize(),
                    'closed': self.closed, 'failed': self.failed}

    def close(self):
        with self.lock:
            self.closed = True
        self.worker.join(timeout=.2)
        return not self.worker.is_alive()
