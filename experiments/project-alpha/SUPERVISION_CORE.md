# Core operator-supervision foundation

The core foundation is now wired to an opt-in supervised HTTP adapter.
Start-ProjectAlpha.ps1 -Supervised uses local renewal and bounded per-window
admission/history; the default timed mode retains its prior limits. FS still
has no authority-renewal tool. See SUPERVISED_HOST_TEST.md.

An explicit local `--supervise-session` core CLI option renews the same active
session and task leases using the existing lifecycle manager. Each individual
lease window is5..1800seconds and remains within the configured renewal horizon.
Default launches still expire. No MCP tool can select this option, renew a lease,
change the workspace or grant authority. Supervision does not extend individual
process timeouts, output/memory budgets or separate approval deadlines.

The trusted Rust service is the supervisor while its owned stdio connection is
alive. It does not infer authorization from host traffic or heartbeat calls.
Revocation remains terminal. A failed renewal permanently disables that timer;
no expired grant is recreated. Closure of stdio explicitly revokes its session
and waits for any already-admitted renewal critical section. Parent UI health
is not proven merely by an open pipe; operator stop and owned pipe closure are
required integration gates for the HTTP adapter.

Native integration proofs:

- supervision_smoke.py: default five-second expiry; explicit supervised service
  survives the initial window without traffic; metadata/scopes stay unchanged;
  reads still work through their renewed leases; cancel remains denied after a
  further timer tick; no remote renewal tool exists.
- supervision_pause_smoke.py: suspend only the test-owned core beyond its lease
  window, resume, require exact inactive-session denials for info and reads,
  then require clean stdio EOF exit. Unix SIGSTOP/SIGCONT and Windows owned
  Popen handles exercise the actual process, not a fake wall-clock adjustment.

Both run on Windows and Linux in the real-link workflow. Local Windows proof
used CI38771044715 artifact11990524756 (source53144e8517ffd6e5741cd40fe9eee30a33c2c66d,
head1d82ea5). Binary SHA256ac20cce0daf5edad2db0b56f7fd686d281d08f473237fb3f7c20ebd761be4678.
The corrected fixture writes exact bytes rather than platform-dependent newlines.
Full exact-head CI is required before publishing a replacement alpha package.

Adapter implementation/acceptance scope: bounded rolling HTTP admission instead of lifetime request
exhaustion; bounded terminal-task retirement instead of ever-growing job maps;
project/journal/output budgets; truthful supervised-mode metadata; core failure,
parent EOF, revoke, disconnect/reconnect and stop integration. These must not be
replaced by an effectively infinite lifetime integer or periodic re-provisioning.
