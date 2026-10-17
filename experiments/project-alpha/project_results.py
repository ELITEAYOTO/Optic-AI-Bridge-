"""Test-client decoder for bounded alpha replies including proof metadata.

The real Rust upstream still uses its separate default16KiB decoder. Alpha
tool payloads <=64KiB plus the fixed proof envelope fit this128KiB client cap.
"""
from real_smoke import structured as upstream_structured


def structured(result):
    return upstream_structured(result, max_bytes=128 * 1024)
