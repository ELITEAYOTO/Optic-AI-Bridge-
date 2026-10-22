# Issue #317 — native owner abrupt-exit comparison

Research-only, 22 October 2026. Branch `bootstrap/issue-317-owned-cleanup-study`.

## Distinct, falsifiable hypothesis

The earlier tests characterized blocking pipe joins, owner-retained uncertainty and Unix group escape. This experiment checks a DIFFERENT failure boundary: a Python native owner exits with `os._exit(79)` after launching a known Rust descendant, without its own `finally`, destructor or pipe-thread joins.

**Windows prediction:** the unnamed kill-on-close Job attached before the Rust fixture resumes will terminate the native descendants when the owning worker exits and the OS closes its sole Job handle. An independent OUTER Job must remain open and reach zero active processes within two seconds *before* it is explicitly terminated.

**Linux prediction:** the worker's new session/process group and private subreaper do not themselves terminate a Rust holder upon worker death. The known holder remains alive, becomes adopted by the independently created PRIVATE outer subreaper, and is explicitly killed using its pinned pidfd then reaped using `waitpid`.

The runnable fixture `issue317_pipe_fixture` is unchanged; the worker/parent use `both` mode. Python reuses the existing experimental `Owner` helper, not production Rust Git code. The independent guard always runs cleanup under finite deadlines, including when assertions fail, and records only OS labels, process exit classification and cleanup completion: **no paths/PIDs in published evidence**.

## Acceptance and limits

The proposed temporary GitHub-hosted native workflow `issue317-abrupt-owner-exit.yml` runs separately under Windows and Ubuntu. Successful source change requires both jobs on its *exact SHA*, real worker exit 79, verified descendant result and confirmed outer cleanup; an incomplete or red job must remain red. No CI/result is preclaimed in this document.

A successful test only distinguishes a native OS lifetime boundary. It does **not** install an owner into `HardenedCommandRunner`, establish Windows cancelable pipe-thread joining, hard CPU reservations, correct Git command session authority, Linux production descendant tracking, or crash recovery in the long-running Bridge. A global subreaper in that application remains unjustified. This is not proof that a Git CAS did not commit, nor closure of #317.

This workflow is intentionally narrow; archive/retire its research-only workflow once durable evidence is documented and an approved owner architecture supersedes it. No `main` edit, no PR/merge, no operator PC, no self-hosted runner, no security or deadline relaxation.
