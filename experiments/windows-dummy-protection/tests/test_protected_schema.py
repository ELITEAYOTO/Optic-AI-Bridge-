import copy
import json
import unittest
from unittest.mock import patch

from persistent_core import FixtureError, MAX_STATE
from protected_registry import decode_metadata


def initial():
    return {'schema': 1, 'owners': {
        'A': {'principal': 'a' * 16, 'epoch': 1, 'version': 0, 'active': True},
        'B': {'principal': 'b' * 16, 'epoch': 1, 'version': 0, 'active': True}}}


class ProtectedSchemaTests(unittest.TestCase):
    def test_bounds_precede_json_and_fixed_error_has_no_input(self):
        with patch('protected_registry.json.loads', side_effect=AssertionError('must not parse')):
            for raw in (None, {}, 'SYNTHETIC-CANARY', b'', b'x' * (MAX_STATE + 1)):
                with self.assertRaises(FixtureError) as error:
                    decode_metadata(raw)
                self.assertEqual(str(error.exception), 'fixture.invalid_state')

    def test_duplicates_unknown_schema_and_malformed_json_are_rejected(self):
        for raw in (b'{"schema":1,"schema":1,"owners":{}}', b'{', b'[]', b'\xff',
                    json.dumps(dict(initial(), schema=2)).encode(),
                    json.dumps(dict(initial(), extra='SYNTHETIC-CANARY')).encode()):
            with self.assertRaises(FixtureError):
                decode_metadata(raw)

    def test_unreachable_and_mistyped_states_are_rejected(self):
        for change in ({'epoch': 2}, {'version': 1}, {'active': False}, {'epoch': True},
                       {'version': True}, {'epoch': 65536}, {'principal': 'SYNTHETIC-CANARY'}):
            data = initial()
            data['owners']['A'].update(change)
            with self.assertRaises(FixtureError):
                decode_metadata(json.dumps(data).encode())

    def test_valid_lifecycle_states_and_snapshot_independence(self):
        for change in ({}, {'epoch': 2, 'active': False},
                       {'epoch': 3, 'version': 1, 'active': True},
                       {'epoch': 65535, 'version': 1, 'active': False}):
            data = initial()
            data['owners']['A'].update(change)
            expected = copy.deepcopy(data['owners'])
            result = decode_metadata(json.dumps(data).encode())
            data['owners']['A']['principal'] = 'c' * 16
            self.assertEqual(result, expected)
