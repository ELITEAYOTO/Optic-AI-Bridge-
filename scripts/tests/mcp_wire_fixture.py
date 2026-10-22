import json
import sys
data = sys.stdin.buffer.readline(4097)
try:
    parsed = json.loads(data.decode("utf-8"))
    valid = parsed == {"probe": 1}
except (ValueError, UnicodeError):
    valid = False
print(json.dumps({"prefix_hex": data[:3].hex(), "valid_json": valid, "length": len(data)}), flush=True)
