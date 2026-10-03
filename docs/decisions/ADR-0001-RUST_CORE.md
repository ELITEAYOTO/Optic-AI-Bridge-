# ADR-0001: Rust Native Core

Status: Accepted

Use Rust for the bridge core/runtime.

Reasons: existing OpticCode reuse, native Windows integration, explicit ownership, no mandatory GC/runtime, and suitability for a small long-lived daemon.

This does not claim Rust automatically prevents logical or resource leaks.
