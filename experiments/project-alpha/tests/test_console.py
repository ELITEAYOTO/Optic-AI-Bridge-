import io
import threading
import unittest
from unittest.mock import patch

from lifecycle_core import Registry
from host_runner import console


class Loop:
    def call_soon_threadsafe(self, callback):
        callback()


class Channel:
    def __init__(self):
        self.messages = []

    def emit(self, value):
        self.messages.append(value)

    def snapshot(self):
        return {'queued': 0}


class Console(unittest.TestCase):
    def exercise(self, text):
        state, stop, channel = Registry(), threading.Event(), Channel()
        with patch('host_runner.sys.stdin', io.StringIO(text)):
            console(state, Loop(), stop, channel)
        self.assertTrue(stop.is_set())
        return state, channel

    def test_stop_survives_control_budget_exhaustion(self):
        state, channel = self.exercise('revoke-a\n' * 40 + 'stop\n')
        self.assertEqual(state.states['A']['epoch'], 17)
        self.assertFalse(state.states['A']['active'])
        self.assertEqual(sum('command_budget_exhausted' in row for row in channel.messages), 24)

    def test_eof_and_malformed_input_fail_closed(self):
        for value in ('', 'stop', 'x' * 65 + '\n', 'proof-stats\n' * 20):
            with self.subTest(value=value[:20]):
                self.exercise(value)

    def test_completed_loop_does_not_raise_from_console(self):
        class ClosedLoop:
            def call_soon_threadsafe(self, callback):
                raise RuntimeError('closed')
        with patch('host_runner.sys.stdin', io.StringIO('stop\n')):
            console(Registry(), ClosedLoop(), threading.Event(), Channel())
