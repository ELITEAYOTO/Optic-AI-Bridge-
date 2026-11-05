# Issue #512 — Immutable upload artifact ID for Windows ChatGPT bundle round-trip

**5 November 2026.** This is a narrow CI reliability correction, not a native GitRead security change, release approval, or workaround for failing tests.

## Failing first-attempt evidence

The exact [failed research CI #42102489478](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/42102489478) on `eb100825d3d620fc3a8c2e828b4f5fa41a7138ab` (run_attempt=1, Windows job `127508616601`) completed unit tests, builds and Node/MCP smokes before artifact round-trip failed. Three other jobs passed.

At 2026-11-05 20:25:03.2030429 UTC, `actions/upload-artifact` reported success for artifact `optic-chatgpt-desktop-node-validation-42102489478-1`, **immutable ID `13449174496`**, size 3,062,040 bytes. At 20:25:03.6242153 UTC, the immediately following `actions/download-artifact` step failed `Artifact not found for name` when resolving that same name. Later, the exact same ID and name were returned by the GitHub artifacts REST API with `expired=false`. The definitive backend cause is **unknown**, and a prior run failure stays failed; never reclassify it as successful or use a rerun-to-green as evidence. This failure signature is distinct from [Node startup #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185).

## Corrective contract on the clean main baseline

This correction is based directly on `main=89ea451d89cb125d6517949c83660465315ff475`, not on the experimental #323 branches. It changes **only** `.github/workflows/ci.yml` (plus this report):

1. Give the existing pinned `actions/upload-artifact@... # v7` step a unique step identifier; **retain** its dynamic original run/attempt name, path, hidden files, `if-no-files-found: error`, and retention settings.
2. Read **the upload action's own `artifact-id` output**. Fail immediately if the receipt is not a strictly positive integer. No automatic retry, fallback to local files, or second upload.
3. Pass that same immutable output to `actions/download-artifact@... # v8` using its documented `artifact-ids` input. The output is scoped to the **current workflow's successful upload**; no name-based immediate artifact search is needed.
4. Keep the original `Verify ChatGPT Desktop Node validation bundle round-trip` step **unchanged**: it checks all mandatory binaries, scripts and plugin files, plus the exact original `source_sha` value in the **downloaded** `SOURCE.txt` against the workflow SHA. The original artifact upload and the downloaded file set must still both actually succeed.

The official `actions/upload-artifact` documentation states that `artifact-id` is the immutable ID returned by a successful upload, and `actions/download-artifact` v8 explicitly supports the `artifact-ids` input:

- https://github.com/actions/upload-artifact#outputs
- https://github.com/actions/download-artifact#download-artifacts-by-id

**Expected benefit:** avoid a name-resolution/listing race at the immediate next step by using the exact returned immutable artifact identity. This is a plausible mitigation, **not an established explanation of GitHub's internal failure**, and it must be verified by a fresh **first-attempt Windows GitHub Actions round-trip**, not only by local static tests.

## Fail-closed local verification

The private, reproducible `_evidence/VERIFY_CI512_CONTRACT.py` parses the actual workflow with PyYAML and ensures: upload ID and upload settings unchanged, receipt passed directly to the downloader, upload/receipt/download/verification order unchanged, no fallback name, and the entire independent `SOURCE.txt` integrity verification block **structurally identical to original `main`**.

It also executes the exact receipt validator in a real Windows PowerShell shell with seven cases: missing, zero, alphabetic, negative and whitespace-altered IDs are **rejected**; a known positive artifact ID is **accepted**. All **7/7** passed. Private contract manifest SHA-256: `2DF09899FB65CF804BE0A9222656C88E954ECBE212E4D5832165239B7D4D8078`.

This local check does **not** substitute for a real GitHub upload/download. Any missing artifact, missing files, wrong source SHA, or unavailable artifact API remains a failing CI check. Do not increase retries or relax content verification to hide a failure.

## Release and integration restrictions

The live GitRead MCP remains unchanged. The V1 native security issue [#323](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/323) and [PR #511](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/511) retain their independent **NO-GO** restrictions. Merge of this CI-only change requires at least a full fresh Windows first-attempt CI on its exact head and a GitHub-confirmed round-trip; post-merge main first-attempt CI is independently checked. An ordinary green CI cannot prove the native-issuer P0 security properties.

## Full first-attempt Windows local regression gates

The full local validation campaign on the clean-main-derived LAB returned exit 0 on all six **first attempts**: pinned Rust 1.99 `cargo fmt --all -- --check`, Windows crate unit suite (**82 passed, 0 failed, 4 ignored**), runtime crate, full workspace, full workspace all-features, and strict Clippy `--workspace --all-targets --all-features -- -D warnings`, with locked/offline Cargo options as applicable. This confirms no local Rust regression; it is **not** evidence of GitHub artifact download availability. Raw first-attempt logs, per-stream SHA-256, and the private supervisor are preserved in `_evidence`. Manifest `FIRST_ATTEMPT_GATES_MANIFEST.json` SHA-256 **`FF4CA87AB277514D615C7F594888EE0203118898573CCB1C0CB8BF9FB0266550`**. A full GitHub Windows round-trip on the exact PR head and independent post-merge `main` remain required.
