from __future__ import annotations

import unittest

from probe_core import CorrelationStore, build_observation, sanitize_headers, sanitize_meta, validate_run_id


class HeaderSanitizationTests(unittest.TestCase):
    def test_sensitive_values_are_not_returned_or_fingerprinted(self) -> None:
        result = sanitize_headers(
            {
                "Authorization": "Bearer top-secret",
                "Cookie": "session=secret",
                "X-Api-Key": "secret-key",
                "Mcp-Protocol-Version": "2026-07-28",
                "X-Request-Id": "abc-123",
            }
        )
        rendered = repr(result)
        self.assertNotIn("top-secret", rendered)
        self.assertNotIn("session=secret", rendered)
        self.assertNotIn("secret-key", rendered)
        self.assertNotIn("authorization", result["names"])
        self.assertNotIn("cookie", result["names"])
        self.assertNotIn("x-api-key", result["names"])
        self.assertEqual(result["safe_values"]["mcp-protocol-version"], "2026-07-28")
        self.assertIn("x-request-id", result["fingerprints"])

    def test_transport_session_id_is_only_fingerprinted(self) -> None:
        result = sanitize_headers({"Mcp-Session-Id": "raw-session-id"})
        self.assertNotIn("raw-session-id", repr(result))
        self.assertIn("mcp-session-id", result["fingerprints"])


class MetaSanitizationTests(unittest.TestCase):
    def test_strings_are_fingerprinted_but_boolean_capabilities_remain_visible(self) -> None:
        result = sanitize_meta(
            {
                "openai/session": "conversation-secret-ish",
                "clientCapabilities": {"elicitation": True, "sampling": False},
            }
        )
        rendered = repr(result)
        self.assertNotIn("conversation-secret-ish", rendered)
        self.assertTrue(result["summary"]["clientCapabilities"]["elicitation"])
        self.assertFalse(result["summary"]["clientCapabilities"]["sampling"])
        self.assertIn("openai/session", result["fingerprints"])


class CorrelationStoreTests(unittest.TestCase):
    def test_same_meta_value_correlates_without_exposing_it(self) -> None:
        store = CorrelationStore()
        first = build_observation(
            request_id=1,
            session_id=None,
            headers={"X-Request-Id": "request-a"},
            meta={"openai/session": "same-chat"},
        )
        second = build_observation(
            request_id=2,
            session_id=None,
            headers={"X-Request-Id": "request-b"},
            meta={"openai/session": "same-chat"},
        )
        one = store.record("run-test-001", first)
        two = store.record("run-test-001", second)
        self.assertEqual(one.call_index, 1)
        self.assertEqual(two.call_index, 2)
        self.assertTrue(two.matching_meta_fingerprints["openai/session"])
        self.assertFalse(two.matching_header_fingerprints["x-request-id"])
        self.assertIsNone(two.same_transport_session_as_first)

    def test_store_is_bounded(self) -> None:
        store = CorrelationStore(max_runs=2, max_events_per_run=2)
        observation = build_observation(
            request_id=1, session_id="s", headers={}, meta={"conversation": "a"}
        )
        store.record("run-test-001", observation)
        store.record("run-test-002", observation)
        store.record("run-test-003", observation)
        result = store.record("run-test-001", observation)
        self.assertEqual(result.call_index, 1)


class RunIdTests(unittest.TestCase):
    def test_validation(self) -> None:
        self.assertEqual(validate_run_id("france-student-01"), "france-student-01")
        for bad in ("short", "contains space", "../escape", "x" * 65):
            with self.assertRaises(ValueError):
                validate_run_id(bad)


if __name__ == "__main__":
    unittest.main()
