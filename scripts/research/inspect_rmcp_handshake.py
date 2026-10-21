"""Read the exact resolved SDK source on GitHub-hosted runners only."""
import json
import os
from pathlib import Path
import re
import tomllib

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Hosted source-audit fixture only")
lock = tomllib.loads(Path("Cargo.lock").read_text(encoding="utf-8"))
version = next(p["version"] for p in lock["package"] if p["name"] == "rmcp")
assert version == "3.4.0", "Revisit source audit on dependency change"
cargo = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
roots = list((cargo / "registry" / "src").glob("*/rmcp-" + version))
assert len(roots) == 1, "Exact SDK source must be unambiguous"
remaining = 340
metadata = []
for source in sorted((roots[0] / "src").rglob("*.rs"), key=lambda p: ("service" not in p.parts, str(p))):
    if source.stat().st_size > 1024 * 1024:
        continue
    lines = source.read_text(encoding="utf-8").splitlines()
    anchors = [i for i, line in enumerate(lines)
               if re.search(r"(async\s+fn\s+serve_server|fn\s+initialize\b|fn\s+get_info\b)", line)]
    covered = set()
    for anchor in anchors:
        start = max(0, anchor - 2)
        end = min(len(lines), anchor + (220 if "service" in source.parts else 120))
        selected = [i for i in range(start, end) if i not in covered][:remaining]
        if not selected:
            continue
        relative = str(source.relative_to(roots[0])).replace("\\", "/")
        metadata.append({"file": relative, "start": selected[0]+1, "end": selected[-1]+1})
        for i in selected:
            print("OPTIC_SDK_SOURCE " + relative + ":" + str(i+1) + " " + lines[i])
        remaining -= len(selected)
        covered.update(selected)
print("OPTIC_SDK_AUDIT " + json.dumps({"rmcp": version, "ranges": metadata, "line_ceiling": 340}))
assert metadata, "Handshake source anchors not located"
