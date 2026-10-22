"""Issue #317: Linux cgroup-v2 delegation feasibility, hosted research only.

This probes only our OWN process cgroup; it never moves processes, writes
cgroup.kill/controller files, or changes an existing parent cgroup. Success
means the test ran, NOT that production confinement is available.
"""
import argparse
import json
import os
from pathlib import Path
import re
import unittest
import uuid

NEEDED = frozenset({"cpu", "memory", "pids"})


def decode_mount(value):
    # Linux mountinfo path escaping, not percent-decoding.
    return re.sub(r"\\([0-7]{3})", lambda m: chr(int(m.group(1), 8)), value)


def cgroup_path(lines):
    found = [line.rstrip("\n")[3:] for line in lines if line.startswith("0::")]
    if len(found) != 1 or not found[0].startswith("/"):
        return None
    return found[0]


def mounts(lines):
    found = []
    for line in lines:
        before, sep, after = line.partition(" - ")
        if not sep:
            continue
        left, right = before.split(), after.split()
        if len(left) < 6 or len(right) < 3 or right[0] != "cgroup2":
            continue
        found.append({
            "root": decode_mount(left[3]),
            "mountpoint": decode_mount(left[4]),
            "rw": "rw" in left[5].split(","),
        })
    return found


def membership_dir(mount, membership):
    root = Path(mount["root"])
    member = Path(membership)
    try:
        relative = member.relative_to(root)
    except ValueError:
        return None
    point = Path(mount["mountpoint"])
    candidate = point.joinpath(relative)
    # Do not follow traversal from a hostile namespace-mount description.
    if ".." in candidate.parts or ".." in member.parts:
        return None
    return candidate


def verdict(status, **observations):
    return {"status": status, "production_confinement_proven": False, **observations}


def read_words(path):
    return set(path.read_text(encoding="ascii").split())


def probe():
    if os.name != "posix" or not Path("/proc/self/cgroup").is_file():
        return verdict("not_linux_cgroup_environment")
    membership = cgroup_path(Path("/proc/self/cgroup").read_text().splitlines())
    if membership is None:
        return verdict("no_unique_unified_membership")
    candidates = mounts(Path("/proc/self/mountinfo").read_text().splitlines())
    matches = [(m, membership_dir(m, membership)) for m in candidates]
    matches = [(m, p) for m, p in matches if p is not None and p.is_dir()]
    if len(matches) != 1:
        return verdict("cgroup_v2_mount_not_unambiguous", mount_candidates=len(matches))
    mount, own = matches[0]
    if not mount["rw"]:
        return verdict("cgroup_v2_read_only")
    if own == Path(mount["mountpoint"]):
        return verdict("mount_root_not_safe_for_probe")
    # Non-root, actually self-owned cgroup only: skip shared/root-owned runner trees.
    stat = own.stat()
    if os.geteuid() == 0 or stat.st_uid != os.geteuid():
        return verdict("no_unprivileged_owned_delegation")
    if not os.access(own, os.W_OK | os.X_OK):
        return verdict("delegation_directory_not_writable")
    if not os.access(own / "cgroup.procs", os.W_OK):
        return verdict("membership_file_not_writable")
    try:
        available = read_words(own / "cgroup.controllers")
        enabled = {x.lstrip("+") for x in read_words(own / "cgroup.subtree_control")}
    except OSError:
        return verdict("controller_state_not_readable")
    missing = sorted(NEEDED - available)
    if missing:
        return verdict("required_controllers_unavailable", missing_controllers=missing)
    missing_enabled = sorted(NEEDED - enabled)
    if missing_enabled:
        # Do not turn controllers on in the parent (it may contain other processes).
        return verdict("required_controllers_not_enabled", missing_controllers=missing_enabled)

    # A fresh empty child under a demonstrably self-owned non-root delegated
    # cgroup is the only permitted write. Do not migrate or signal a process.
    name = "optic-issue317-probe-" + uuid.uuid4().hex
    child = own / name
    created = False
    try:
        child.mkdir(mode=0o700)
        created = True
        if read_words(child / "cgroup.procs"):
            return verdict("new_cgroup_not_empty")
        needed_files = ["cgroup.kill", "cpu.max", "memory.max", "pids.max"]
        if not all((child / item).exists() for item in needed_files):
            return verdict("child_missing_kernel_limits_or_kill_interface")
        return verdict("delegation_candidate_only", empty_child_created=True)
    except OSError as exc:
        # Publish errno only, never runner paths, exceptions or PID.
        return verdict("isolated_child_creation_denied", errno=exc.errno)
    finally:
        if created:
            # cgroupfs directories are kernel objects; remove only our own empty child.
            child.rmdir()


class ParserTests(unittest.TestCase):
    def test_unified_path(self):
        self.assertEqual(cgroup_path(["0::/session.slice/thing"]), "/session.slice/thing")
        self.assertIsNone(cgroup_path(["1:name=systemd:/x"]))
        self.assertIsNone(cgroup_path(["0::/x", "0::/y"]))

    def test_mount_and_root_mapping(self):
        sample = "61 20 0:29 /user.slice /sys/fs/cgroup rw,nosuid,nodev - cgroup2 cgroup2 rw"
        mount = mounts([sample])[0]
        self.assertEqual(membership_dir(mount, "/user.slice/job"), Path("/sys/fs/cgroup/job"))
        self.assertIsNone(membership_dir(mount, "/other/job"))

    def test_read_only_mount_and_escaped_path(self):
        sample = "61 20 0:29 / /sys/fs/cgroup ro,nosuid,nodev - cgroup2 cgroup2 ro"
        mount = mounts([sample])[0]
        self.assertFalse(mount["rw"])
        self.assertEqual(decode_mount("/some\\040space"), "/some space")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.main(argv=[__file__], exit=False, verbosity=2)
        if not result.result.wasSuccessful():
            raise SystemExit(1)
        return
    if not args.output:
        parser.error("--output is required")
    result = {
        "sha": os.environ.get("GITHUB_SHA"),
        "run": os.environ.get("GITHUB_RUN_ID"),
        "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "linux_kernel_research_only": True,
        "completed": False,
        "probe": None,
    }
    path = Path(args.output)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        result["probe"] = probe()
        result["completed"] = True
    finally:
        path.write_text(json.dumps(result, indent=2), encoding="utf-8")
        print("OPTIC_317_CGROUP_PREFLIGHT " + json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    main()
