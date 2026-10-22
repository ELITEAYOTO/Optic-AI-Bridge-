import json
import sys
data = sys.stdin.buffer.readline(4097)
try:
    parsed = json.loads(data.decode("utf-8"))
    valid = parsed == {"probe": 1, "unicode": "\u00e9\u96ea"}
except (ValueError, UnicodeError):
    valid = False
response = {"prefix_hex": data[:3].hex(), "valid_json": valid, "length": len(data),
            "echo": "\u00e9\u96ea"}
sys.stdout.buffer.write((json.dumps(response, ensure_ascii=False) + "\n").encode("utf-8"))
sys.stdout.buffer.flush()
