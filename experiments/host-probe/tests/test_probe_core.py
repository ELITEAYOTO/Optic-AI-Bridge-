from __future__ import annotations

import unittest

from probe_core import (
    CorrelationStore,
    build_observation,
    sanitize_headers,
    sanitize_meta,
    stable_fingerprint,
    validate_run_id,
)


class HeaderSanitizationTests(unittest.TestCase):
    def test_sensitive_values_are_not_returned_or_fingerprinted(self) -> None:
        result = sanitize_headers(
            {
                "Authorization": "Bearer top-secret",
                "Cookie": "session=secret",
                "X-Api-Key": "secret-key",
                "X-Auth-Mode": "private-auth-value",
                "X-Password": "private-password",
                "Mcp-Protocol-Version": "2026-07-28",
                "X-Request-Id": "abc-123",
            }
        )
        rendered = repr(result)
        self.assertNotIn("top-secret", rendered)
        self.assertNotIn("session=secret", rendered)
        self.assertNotIn("secret-key", rendered)
        self.assertNotIn("private-auth-value", rendered)
        self.assertNotIn("private-password", rendered)
        self.assertNotIn("authorization", result["names"])
        self.assertNotIn("cookie", result["names"])
        self.assertNotIn("x-api-key", result["names"])
        self.assertNotIn("x-auth-mode", result["names"])
        self.assertNotIn("x-password", result["names"])
        self.assertEqual(result["safe_values"]["mcp-protocol-version"], "2026-07-28")
        self.assertIn("x-request-id", result["fingerprints"])

    def test_transport_session_and_environment_headers_are_only_fingerprinted(self) -> None:
        result = sanitize_headers(
            {
                "Mcp-Session-Id": "raw-session-id",
                "Origin": "https://private.example",
                "User-Agent": "identifying-client/123",
            }
        )
        rendered = repr(result)
        self.assertNotIn("raw-session-id", rendered)
        self.assertNotIn("https://private.example", rendered)
        self.assertNotIn("identifying-client/123", rendered)
        self.assertIn("mcp-session-id", result["fingerprints"])
        self.assertIn("origin", result["fingerprints"])
        self.assertIn("user-agent", result["fingerprints"])


class MetaSanitizationTests(unittest.TestCase):
    def test_strings_and_numbers_are_fingerprinted_but_boolean_capabilities_remain_visible(self) -> None:
        result = sanitize_meta(
            {
                "openai/session": "conversation-secret-ish",
                "account/id": 123456789,
                "clientCapabilities": {"elicitation": True, "sampling": False},
            }
        )
        rendered = repr(result)
        self.assertNotIn("conversation-secret-ish", rendered)
        self.assertNotIn("123456789", rendered)
        self.assertTrue(result["summary"]["clientCapabilities"]["elicitation"])
        self.assertFalse(result["summary"]["clientCapabilities"]["sampling"])
        self.assertIn("openai/session", result["fingerprints"])
        self.assertEqual(result["summary"]["account/id"]["type"], "int")

    def test_unusual_meta_keys_are_not_echoed_raw(self) -> None:
        result = sanitize_meta({"user@example.com": "value"})
        rendered = repr(result)
        self.assertNotIn("user@example.com", rendered)
        self.assertTrue(any(key.startswith("key#") for key in result["keys"]))

    def test_fingerprint_is_stable_within_one_probe_process(self) -> None:
        self.assertEqual(stable_fingerprint("same"), stable_fingerprint("same"))
        self.assertNotEqual(stable_fingerprint("same"), stable_fingerprint("different"))


class ObservationTests(unittest.TestCase):
    def test_missing_request_id_is_explicit_and_not_fingerprinted(self) -> None:
        result = build_observation(request_id=None, session_id=None, headers={}, meta={})
        self.assertFalse(result["request_id_present"])
        self.assertIsNone(result["request_id_fingerprint"])

    def test_present_request_id_is_fingerprinted(self) -> None:
        first = build_observation(request_id=42, session_id=None, headers={}, meta={})
        second = build_observation(request_id=42, session_id=None, headers={}, meta={})
        different = build_observation(request_id=43, session_id=None, headers={}, meta={})
        self.assertTrue(first["request_id_present"])
        self.assertIsNotNone(first["request_id_fingerprint"])
        self.assertEqual(first["request_id_fingerprint"], second["request_id_fingerprint"])
        self.assertNotEqual(first["request_id_fingerprint"], different["request_id_fingerprint"])


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

    def test_appeared_or_disappeared_fingerprint_is_reported_as_changed(self) -> None:
        store = CorrelationStore()
        first = build_observation(
            request_id=1,
            session_id=None,
            headers={"X-Stable": "same", "X-Gone": "gone"},
            meta={"stable": "same", "gone": "gone"},
        )
        second = build_observation(
            request_id=2,
            session_id=None,
            headers={"X-Stable": "same", "X-New": "new"},
            meta={"stable": "same", "new": "new"},
        )
        store.record("run-union-001", first)
        result = store.record("run-union-001", second)
        self.assertTrue(result.matching_header_fingerprints["x-stable"])
        self.assertFalse(result.matching_header_fingerprints["x-gone"])
        self.assertFalse(result.matching_header_fingerprints["x-new"])
        self.assertTrue(result.matching_meta_fingerprints["stable"])
        self.assertFalse(result.matching_meta_fingerprints["gone"])
        self.assertFalse(result.matching_meta_fingerprints["new"])

    def test_store_is_bounded_by_run_count(self) -> None:
        store = CorrelationStore(max_runs=2, max_events_per_run=2)
        observation = build_observation(
            request_id=1, session_id="s", headers={}, meta={"conversation": "a"}
        )
        store.record("run-test-001", observation)
        store.record("run-test-002", observation)
        store.record("run-test-003", observation)
        result = store.record("run-test-001", observation)
        self.assertEqual(result.call_index, 1)

    def test_event_limit_fails_without_replacing_original_baseline(self) -> None:
        store = CorrelationStore(max_runs=2, max_events_per_run=2)
        first = build_observation(
            request_id=1, session_id="session-a", headers={}, meta={"conversation": "first"}
        )
        second = build_observation(
            request_id=2, session_id="session-a", headers={}, meta={"conversation": "second"}
        )
        third = build_observation(
            request_id=3, session_id="session-b", headers={}, meta={"conversation": "third"}
        )
        store.record("run-limit-001", first)
        two = store.record("run-limit-001", second)
        self.assertEqual(two.call_index, 2)
        with self.assertRaises(ValueError):
            store.record("run-limit-001", third)
        self.assertTrue(store.reset("run-limit-001"))
        after_reset = store.record("run-limit-001", third)
        self.assertEqual(after_reset.call_index, 1)


class RunIdTests(unittest.TestCase):
    def test_validation(self) -> None:
        self.assertEqual(validate_run_id("france-student-01"), "france-student-01")
        for bad in ("short", "contains space", "../escape", "x" * 65):
            with self.assertRaises(ValueError):
                validate_run_id(bad)


if __name__ == "__main__":
    unittest.main()
