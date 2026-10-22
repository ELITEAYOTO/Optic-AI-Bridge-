# Issue #317 — cgroup v2 delegation gate (Linux hosted preflight)

**Research only · 22 October 2026 · not production code.**

## Question

Can the *current unprivileged Linux runner* establish a dedicated cgroup v2 boundary with CPU, memory, pids controls and a group-kill interface, without broad privileges, changing an existing parent, or moving an unowned process? The preceding proof [#39707390217](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39707390217) showed that an ordinary Linux process group leaves a synthetic Rust descendant alive after its owner exits; the independent subreaper reclaimed it. That supervisor is not portable production containment.

The Linux kernel documents cgroup v2 **delegation** and group-wide `cgroup.kill` separately from simply mounting the filesystem. The test preflight follows [the kernel's cgroup v2 documentation](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html). Merely having `/sys/fs/cgroup` and a `cgroup.kill` file is **not** proof of delegated write authority, process migration, controller enforcement or all-descendant containment.

## Probe scope and semantics

`scripts/research/issue317_cgroup_v2_preflight.py` reads only `/proc/self/cgroup`, `/proc/self/mountinfo` and kernel cgroup interface files. A pure-parser test checks mount mapping and rejects incomplete mount descriptions. The probe reports only a **status category and safe booleans** (no paths/PIDs).

The only permitted mutation is creating and immediately removing a **fresh, empty, unique child** beneath the *current process's own* cgroup, and only after verifying all of these: one unambiguous cgroup2 mount; mount writable; target is not mount root; current process **non-root**; target owned and writable by the same effective user; `cgroup.procs` writable; `cpu`, `memory`, `pids` available and already activated for children. If any check fails, stop with a negative status and **do not create a cgroup**. No writes to `cgroup.procs`, `cgroup.subtree_control`, resource limit files, `cgroup.kill`, or global system control. A failed mkdir is reported as denied. A successful empty child and required files yields only `delegation_candidate_only`, **not** containment proof.

**Expected normal outcome:** many shared CI runners will be `cgroup_v2_read_only` or `no_unprivileged_owned_delegation`. A green GitHub workflow means the probe executed and was classified accurately; it never means those missing capabilities were obtained. If the preflight returns a negative status, the next production design must retain a fail-closed unsupported path rather than relying on host-cgroup privileges. An environment-specific outcome cannot establish that every Linux installation behaves identically.

## Acceptance/limits

The dedicated GitHub-hosted Ubuntu workflow runs static parsing assertions plus the narrow preflight, emits exact SHA/run/attempt and preserves its JSON for seven days, including negative/unsupported categories. **Windows requires no cgroup experiment**; its separate native Job research already has independent proofs.

This preflight intentionally does **not** create a native child process, migrate a PID, validate `cgroup.kill` efficacy, prove controller enforcement, design delegated systemd scopes, or authorize a Linux native owner. A future end-to-end process-tree test requires an explicitly isolated and delegated environment, pre-spawn reservation, native all-descendant cleanup and independent supervision. Resource/authority rules remain as in [ISSUE317_NATIVE_OWNER_INTEGRATION_CONTRACT_2026-10-22.md](ISSUE317_NATIVE_OWNER_INTEGRATION_CONTRACT_2026-10-22.md). No code changes in `HardenedCommandRunner`, `main`, or PR #318.

**Result at file creation:** pending native hosted GitHub CI; do not infer result until exact-SHA workflow logs are examined.
