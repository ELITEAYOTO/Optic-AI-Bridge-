"""Independent bounded decoders for Rust snapshots and host-visible alpha replies."""
from real_smoke import structured as decode
from project_schema import MAX_CONTENT, file_limit


def upstream(result, max_file_bytes=MAX_CONTENT):
    # One full Rust snapshot includes base64 for cap+1 bytes to detect oversize.
    # Default16KiB is unchanged; 32KiB files need a finite larger internal envelope.
    cap = file_limit(max_file_bytes)
    return decode(result, max_bytes=max(16384, 4 * ((cap + 3) // 3) + 1024))


def structured(result):
    # Host payload cap64KiB plus fixed proof metadata; this is not a file budget.
    return decode(result, max_bytes=128 * 1024)
