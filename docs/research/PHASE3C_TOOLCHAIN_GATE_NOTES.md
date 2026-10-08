# Phase 3C toolchain gate notes

This note exists to preserve the narrow scope of the current characterization work.

- Do not change public policy admission in this gate.
- Do not add broad workspace read/write authority to make a tool pass.
- Do not add network authority.
- Do not treat `--version` success as proof that builds/tests are safe or usable.
- Treat inability to read an explicitly granted file as evidence to investigate the exact dependency (cwd, directory traversal/enumeration, runtime libraries, stdlib/sysroot/JDK, registry, scratch/output) before changing authority.
- An ungranted-file read is a security failure and must fail CI.
- Keep the root working tree untouched; development uses the dedicated Phase 3C worktree/branch.
