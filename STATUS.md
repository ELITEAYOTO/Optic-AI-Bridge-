# Current project status

## Windows PowerShell acceptance coverage

A real Desktop5.1 doctor smoke exposed UTF-8 BOM bytes before initialize.
The doctor now starts redirected MCP streams with BOM-free UTF-8; the Framework
fallback restores console encoding after Start, including failure. Native byte,
Unicode, pipe and real Bridge/isolated-Node checks accompany current pwsh coverage.
The test worker adapts Framework argument quoting and records shell version/edition. These checks are not Windows10/11 installation certification.
[Acceptance scope and remaining risks](docs/engineering/WINDOWS_POWERSHELL_ACCEPTANCE.md).


## Release source provenance guard — 22 October 2026

The tag workflow now requires the exact source to be integrated into main and
its latest matching independent main CI to be complete/successful, including
native Windows, Ubuntu and dependency jobs. Missing, pending or ambiguous evidence
fails closed before release build/publication. This does not authorize a release
or certify V1 readiness. [Contract and limitations](docs/engineering/RELEASE_SOURCE_GATE.md).

#317 has a new **research-only** native Rust owner slice (8 cases), not a production
runner correction: [verified checkpoint](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/317#issuecomment-6332249104).
Stdin ownership, resource/session registry, revoke/seal/reap and CAS remain gates.


## Windows V1 stabilization — operator direction, 22 October 2026

Windows10/11 is the V1 target; supported OS/PowerShell combinations still require
their own acceptance. Hosted Windows Server checks are native Windows evidence,
not proof that every Windows10/11 installation works. Functional Linux delivery
is post-V1; existing Ubuntu Rust/dependency checks remain required regression guards.

The #318 change rejects non-octal raw Git modes and bounds Windows doctor stderr
capture after EOF/timeout using a sole-consumer native pipe snapshot. Tests preserve
primary failures, cap bytes/messages, reject unknown labels and supervise all fixture
processes. No startup deadline, full executable identity/pin, resource/permission,
AppContainer or network boundary is relaxed. Exact integration status and source
checks are in [PR318](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/318);
the [native proof](docs/research/WINDOWS_DOCTOR_PIPE_LIFETIME_2026-10-22.md)
distinguishes demonstrated error-path failures from startup hypotheses.

**Still open:**319 natural first MCP30s timeout (cause not established),185 separate
Node/AppContainer10s failure,317 demonstrated generic command pipe-join overrun
(no production Rust ownership correction),176/182 sensitive/provenance boundaries.
These remain V1/release gates where applicable. An unrelated Git/parser correction
and a green current CI do not certify startup reliability or real-workspace safety.
No feature unfreeze, Electron, generalized Cargo, alpha merge or user-PC execution.

Older checkpoints below describe their dates, not current branch heads.

## Historical verified checkpoint — #314 (2026-10-21)

**Verified source of truth:** `main=7bc17c16a52383bf6fb7ba68dc4b9348b259be9a`, 0 open PRs. [PR #314](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314) is squash-merged: a hard-linked file can bear both a classified sensitive name (`.env`) and an innocent-looking alias (`README.txt`); `fs_read` now requires existing matching `ReadSensitiveScope` in addition to ordinary FileRead for multi-linked opened native identities. This is intentionally conservative for harmless hard links. Native Windows/Linux regression tests passed on accepted source SHA `fbf7c9f40bbae18d4017f237b8783a19964620c8` in [CI #39576724127](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39576724127); three specialized native workflows also passed. Independent [push CI #39577461324](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39577461324) passed **first attempt on the exact squash SHA**, Windows Rust, Linux Rust and dependencies. Earlier rustfmt failure on a superseded PR head remains preserved. [Complete proof](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/314#issuecomment-6312779381).

**Residual trust boundaries:** #176 (arbitrary secrets in ordinary files, native ancestor identity/races), #182 (Git index/commit content provenance and branch/ref metadata), and #185 (intermittent 10-second AppContainer Node timeout of unknown cause) remain **OPEN**. A separate ~30-second first MCP response flake remains unproven. [Targeted GitRead review](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182#issuecomment-6312922336) confirmed existing status/diff protections and recorded that status `# branch.head` / `# branch.upstream` values are returned as Git metadata without separate sensitivity classification; this does **not** prove credential leakage. Do not change the public metadata policy without an explicit contract and synthetic regression.

**Execution:** pre-alpha, feature work frozen, no real-project remote access, no alpha/generated-project-editing merge, no user-PC or self-hosted execution, no privilege/sandbox/timeout relaxation. Source head must always be rechecked live. All later candidate checkpoints below are dated history, not unmerged current work.

## Verified Policy B path-floor integration — after #310 (2026-10-21)

**Source of truth verified on GitHub:** `main=bd2915c3a9833c1da5a8fc5005445d1884260d4f`; [#310](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/310) is squash-merged; zero PRs are open at this checkpoint. Independent [main CI #39552523257](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39552523257) succeeded **first attempt** at the exact merge SHA on native Windows, Ubuntu and dependency policy. #310's accepted head `154d7ea6ab6b10aa967ab0a2f2034f2f06400fe9` passed [source CI #39551993433](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993433) and [native preparation #39551993151](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39551993151) first attempt. Earlier red PR CI attempts remain evidence of corrected rustfmt and verified Git raw-protocol test failures; do not overwrite them.

**Integrated security path floor:** #308 adds shared Rust-core sensitive-name classification and the FileRead / fs_list additional scoped authority checks. #309 parses Git porcelain-v2 `-z` metadata and denies unknown or unauthorized protected names. #310 produces `git diff --raw -z --patch` in a single hardened process, validates NUL-delimited raw changed paths **and the extra NUL before the patch**, denies unverifiable streams, and requires active typed FileRead (ReadContent + ReadSensitive) authority for classified protected patch contents while read admission remains held. No new default grants, remote transport, feature expansion or weaker containment. See [ADR-0034](docs/decisions/ADR-0034-SENSITIVE-READ-CLASSIFICATION.md).

**Still blocked:** lexical filename rules cannot prove ordinary-named contents are secret-free (#176); Git metadata and diff coverage remain subject to concurrent repository changes, unproven alternate path identity and other provenance/authority guarantees (#182). Windows Node/AppContainer timing #185, separate MCP first-response intermittency, crash recovery atomicity/persistence, ancestor races, stress/fault tests and incomplete code audit remain. #176 and #182 are OPEN; #176 was accidentally auto-closed during #310 merge by a GitHub closing keyword in prose and explicitly reopened with evidence. No real-workspace remote exposure or feature unfreeze. Older sections marked *candidate* below are **historical pre-merge records** and are superseded by this verified checkpoint; do not reapply earlier base SHAs.

## Git diff path-verified output boundary — candidate (2026-10-21)

Live GitHub before branching: `main=ccaa7c05af27cfaab02d5dcb93369f60a0c01f53`; [#309](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/309) squash-merged; independent [post-merge CI #39549267903](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39549267903) green first attempt (native Windows/Linux/dependencies); zero open PRs. New **candidate** `security/git-diff-raw-output-authority`: not integrated until exact-PR-SHA CI and independent main CI succeed.

Proven #182 gap: unlike the now-protected `git_status`, `git_diff` returned raw staged/worktree patch bytes without classifying any changed path. Candidate correction: one hardened Git invocation `--raw -z --patch` with raw changed-path verification and strict rejection of malformed/unknown/native-unrepresentable outputs. No preflight/patch race from two separate Git calls. For protected patch paths, check an additional active typed FileRead effect (both content and sensitive scopes) **inside session admission**, then release only the patch suffix. Normal source patches stay available. Unit/MCP regressions target unscoped/path-scoped/dir-scoped/staged protected patches, ordinary code, malformed raw and resource budgets.

**Limit:** Git diff source metadata and patch come from the same Git diff command, but worktree snapshots are not atomic against arbitrary concurrent edits. Lexical path rules do not detect secrets in normal-looking file contents. #176/#182 remain OPEN; no remote real-project deployment or restart of new features.

## Git status sensitive-path classification — candidate (2026-10-21)

Live baseline for this increment: `main=bfba4b97a531440de08012ca50106797d995646b`, [#308](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/308) squash-merged, independent [post-merge CI #39546153987](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39546153987) green first attempt, zero open PRs. Operator-approved Policy B remains binding, with no remote exposure to real workspaces.

**Candidate `security/git-status-sensitive-paths` (unmerged until exact-head CI):** validate every path in the returned Git porcelain-v2 `-z` status output, including rename original names, before returning status bytes. Reuse the Rust-core classifier and require an active matching sensitive read lease in the admitted MCP worker; refuse malformed/unknown/unrepresentable records. Native Windows/Linux tests cover sensitive staged and untracked files, mixed normal content, explicit grants, prefix mismatches and invalid output. No new CLI, transport, grant, override or runtime privilege.

**Open:** #182 remains blocked by the *independently unclassified raw `git_diff`*; harmless-looking files can carry secrets and #176 remains incomplete. Do not close either issue or enable real-workspace remote access. Inspect GitHub CI and final SHA before considering this candidate merged.

## Sensitive-read authority — policy B approved, first scoped fix under CI review (2026-10-21)

After merged #307, live `main=3d9a0f78304578af53fd23b9fed7d08603348e0b`, zero open PRs, and post-merge hosted Windows/Linux/dependency CI #39542505029 passed first attempt. The operator has now approved **Policy B**: shared deterministic classification of sensitive path/content across FileRead, FileSearch and GitRead, with fail-closed handling of unverifiable outputs. **This paragraph describes a candidate increment on `security/sensitive-file-authority-scope`, not a verified merge.**

First narrow corrective implementation: pure conservative sensitive-path classifier in core, enforced by `PolicyEngine` for file reads (ordinary content and matching sensitive scopes both required), and by `fs_list` before releasing sensitive directory names (whole affected page denied without additional sensitive grant). Unit and MCP regressions cover path ancestry, case, absent/mismatched sensitive leases, ordinary content and explicit scoped access. Exact-head hosted Windows/Linux acceptance and independent main CI are **not yet claimed**.

**Important limitation:** lexical name detection is not secret-content inspection. `GitRead` status/diff remain outside this increment and can independently disclose names or secret-bearing patches. Both #176 and #182 remain **open blockers** to real-project remote/autonomous exposure. Do not infer safe remote access, close these tickets, or restart features. Next correction must treat Git output records and unverifiable outputs fail-closed under the **same** classifier (ADR-0034) with exact SHA CI. No broad grants, deadline changes, PC execution or alpha merge.

## Successor handoff snapshot — 2026-10-21, after merged #306

GitHub live before this documentation handoff: `main=dadaf48539788235b689dd4e09adcd2fb94904f3`, **zero PRs open**, [PR #306](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/306) squash-merged (governance and stabilization exit-gates documentation). [PR #306 source CI #39540522392](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540522392) passed first attempt, and independent [main CI #39540940136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39540940136) passed first attempt on exact merge SHA (hosted Rust Windows, Rust Linux, dependencies). This is **dated evidence, not an instruction to restore a commit**. Verify live GitHub for changes after this handoff's own PR.

**Next step:** security source audit of the `fs_read`/`fs_list` vs `GitRead`/status/diff data-exposure contract, [#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) and [#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182), with **no sensitive classifier yet proven** and no permission broadening. Other open reliability boundaries remain [#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) Windows 10s Node/AppContainer, distinct MCP ~30s initial response, recovery atomicity/ancestor races/power-loss durability, soak/fault tests and incomplete exhaustive code audit. Administrative issues #180/#181/#183 have code-level regressions/protections recorded and need acceptance reconciliation, not automatic recreation or closure.

**Operating rule:** one PR at a time only after proving a defect, tests+docs together, exact SHA CI and independent post-fusion main verification; no operator PC, self-hosted runner, new features, release or changing GitHub branch protections without approval. Full future-chat instructions live in [PROJECT_CONTINUITY_PROMPT.md](PROJECT_CONTINUITY_PROMPT.md) and [GITHUB_HANDOFF.md](docs/engineering/GITHUB_HANDOFF.md); old candidate headings below are archive records.


## Verified integration checkpoint — 2026-10-21 (before this documentation update)

At the **live GitHub check**, `main` was `43d3a4aeedee04dd03e895e27e81d3bc5af6652c`, with **zero open PRs**. Independent push [CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed **first attempt** on this exact commit (hosted Windows Rust, Linux Rust, dependency policy). This is a dated observation, **not** an instruction to reset or a claim about later commits.

The recent stabilization work is **already merged**:

| PR | Main merge SHA | Narrow effect and verified integration |
| --- | --- | --- |
| [#301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) | `05b5a928` | Preflight all unpublished recovery artifacts before deleting; [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) passed first attempt. Source CI #39529419994 needed an unchanged-SHA Windows job retry after the known 10s Node/AppContainer failure. |
| [#302](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/302) | `c8c1110f` | Documentary continuity; [main CI #39530925055](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530925055) passed first attempt. |
| [#303](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/303) | `9c6c9109` | Canonical directory scope before `fs_list` authorization; [main CI #39534614408](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39534614408) passed first attempt. |
| [#304](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/304) | `9d90485f` | Reserve canonical `.git` control paths from generic mutation; [main CI #39536541153](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39536541153) passed first attempt. |
| [#305](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/305) | `43d3a4ae` | Fit the final `git_log` response **including** newly added continuation cursor; [main CI #39538634249](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39538634249) passed first attempt. |

**Not closed by green CI:** Windows Node/AppContainer 10-second no-output intermittency ([#185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185)); separate ~30-second MCP first-response intermittency; recovery multi-delete non-atomicity, ancestor namespace races and Windows power-loss durability; sensitive-file read classification ([#176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176)) and GitRead sensitivity composition ([#182](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/182)). #180/#181/#183 still appear **open administratively** despite already-integrated scoped corrections/tests; reconcile their *remaining acceptance criteria*, do not repeat the implementation or silently close them.

The core is **pre-alpha**. The feature freeze and GitHub-hosted-only restrictions remain. See [stabilization exit gates](ROADMAP.md), [current handoff](docs/engineering/GITHUB_HANDOFF.md) and [reliability debt](docs/engineering/RELIABILITY_DEBT.md). The older sections below are **dated historical reports**, including wording such as “candidate” written before those PRs merged.


## Git-log response size & pagination — bounded regression

A deterministic `git_log` response-fitting flaw existed when the final history page had `next_cursor: None`: trimming entries to meet the structured response-byte limit then **added** a cursor containing the snapshot HEAD and new offset without checking the resulting serialized size. A too-large MCP response could be emitted despite the configured output budget. The narrow fix repeatedly refits after cursor insertion, reducing the page while preserving at least one advancing entry; if no entry plus cursor fits, it fails closed with `optic.response_too_large`. Cursor arithmetic now rejects overflow rather than saturating into a non-advancing token.

Focused tests construct an end-of-history page with a carefully selected byte budget to prove rejection when the cursor cannot fit, acceptance when it can and preservation of a complete cursor-free last page. GitRead capabilities, HEAD snapshot pinning, Git subprocess sandbox and resource limits are unchanged. **Verify the actual PR, exact-head CI, and independent `main` CI before claiming this change is integrated.** No new feature, user-PC activity or CI bypass.


## Git-control mutation authority boundary — stabilization candidate (2026-10-21)

The [security audit #180](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/180) identified a real authority-composition gap: generic FileWrite/FileDelete with whole-workspace or prefix grants could address `.git` control metadata, despite dedicated GitIntegrate authority. The scoped candidate reserves every exact `.git` path component (case-insensitively, including linked-worktree `.git` files) from generic transactional write/delete/patch preparation, **after canonical target observation**; generic directory creation rejects the same namespace. It does not expand session/lease rights or change the Git integration runtime. Ordinary dotfiles such as `.gitignore` remain regular project files.

Cross-platform preparation tests, a Unix internal alias test, native Windows end-to-end authorized mutation and directory tests, and explicit MCP denial-code tests are included. **Do not describe as delivered until exact-head GitHub-hosted Windows/Linux/dependency/native-preparation CI and post-merge main checks succeed.** Parent path swaps, hardlink alias identity and power-loss durability remain independent issues. Never infer that sensitive-content reads or remote exposure are solved by this change. No new features, permission expansion, operator-PC activity or self-hosted CI.


## Scoped fs_list canonical-target authorization — security stabilization (2026-10-21)

Source review of the existing local MCP `fs_list` found a concrete lexical/canonical scope mismatch: policy evaluated a requested directory alias before `BoundedFileSystem::list` followed the alias to a different workspace subtree. Under prefix-scoped `SearchMetadataScope`, this permits metadata enumeration outside the granted prefix. The bounded correction resolves the **canonical directory target before policy evaluation** and lists that resolved canonical root; returned paths identify the actual directory. A Unix symlink regression distinguishes restricted-prefix denial from whole-workspace allowed canonical listing; a portable root/normal-directory regression runs on hosted Windows and Linux.

The change does not grant new permissions, create a transport, add a sensitive-content classifier, pin ancestor directory handles, or eliminate concurrent namespace replacement races. Review the exact source PR and native CI before claiming delivery. [Security issue #176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/176) remains open for its separate sensitive-read classifier and remaining path/race boundaries; #301 recovery and Node #185 remain independent. Feature freeze remains in force.


## Stabilization: unpublished recovery evidence — PR #301 (2026-10-21)

[PR #301](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/301) was squash-merged into `main` at `05b5a928c862601926e8b0cc11540ce020caf271`. Its scoped fix preflights **all** unpublished `*.prepared.tmp` entries with existing native opened-file, single-link and no-reparse checks before the first deletion, and rechecks each immediately before removal. The new ordered-fixture regression proves a later hard-linked unsafe entry cannot erase earlier unrelated evidence; repair permits normal cleanup. Windows and Linux logs explicitly record `later_aliased_unpublished_entry_preserves_all_earlier_evidence ... ok` on accepted source `089e64a2b6c95dc0705eeaaa22f90194dfb7be66`.

[Source CI #39529419994](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529419994) passed after **attempt 2**, not on first attempt. Initial Windows attempt failed in the already-known native Node/AppContainer 10s timeout (no captured launcher/Node marker); three serial same-head diagnostics passed and remain evidence in [issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185#issuecomment-6306039365). Linux and dependency checks passed on the original attempt; [native preparation #39529420240](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39529420240) passed first attempt. A failed-Windows-job-only rerun passed the full native suite and smokes on unchanged SHA. Independent [main CI #39530161176](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39530161176) **passed first attempt on exact squash SHA** `05b5a928c862601926e8b0cc11540ce020caf271` (hosted Windows Rust, Linux Rust and dependency policy).

**Limit:** This closes one demonstrable early-cleanup-on-later-unsafe-entry defect. It does **not** guarantee atomicity of multiple deletions if a later actual deletion fails, concurrent ancestor/path identity, Unix unlink race safety or Windows power-loss directory persistence. Node/AppContainer (~10s) and separate MCP first response (~30s) intermittencies remain open. Feature freeze and GitHub-only execution restrictions unchanged.



## Stabilisation Windows — PR #299 fusionnée le 21 octobre 2026

À l'observation GitHub après la fusion : `main=6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea`, aucune PR ouverte. La [PR #299](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/299) a été fusionnée par squash depuis la tête exacte `aca382535f24b6f6b1c26c2313302d315099ed4e`. Sa [CI source #39526324369](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324369) a réussi **du premier coup** (Windows Rust natif, Linux Rust, dépendances), de même que la [préparation native #39526324493](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526324493). La [CI indépendante post-fusion #39526677098](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39526677098) a **réussi dès la première tentative**, sur le SHA exact `6fe41ba2ce0e66f245acb7ea7c6c253e3f0f77ea` (Windows Rust, Linux Rust et dépendances). Les docs sont un relevé daté, jamais la tête GitHub live.

**Incrément réel :** jalons monotones opt-in, à libellés fixes, écrits sur `stderr` par le lanceur d'isolation Windows : entrée, requête, entrées validées, profil, grants, enfant créé suspendu, token vérifié, reprise, fin. `OPTIC_LAUNCHER_PHASES=1` est transmis uniquement en tant que constante opt-in à ce lanceur, dans la preuve Windows existante et les répétitions diagnostiques déclenchées lors d'un échec. Le test natif vérifie la présence des phases et conserve le refus réseau effectif, la sortie attendue, les limites de 10 secondes et les règles d'isolation.

**Exemple validé :** `target_created_suspended=14 ms`, `target_resumed=14 ms`, `target_exited=361 ms` à partir de l'entrée du lanceur. Le parent a séparément observé `launch_returned=13 ms`, `terminal_observed=407 ms` ; ces horloges ont des origines différentes. Ces données prouvent le fonctionnement de l'instrumentation sur un essai **réussi**, pas la cause des expirations précédentes. Les intermittences Windows AppContainer/Node de ~10s et MCP première réponse de ~30s restent **ouvertes**. Le nettoyage multi-fichiers, les courses d'ancêtres et la persistance après coupure restent non validés. Aucun correctif fonctionnel de ces défauts n'est revendiqué. Le gel des nouvelles fonctions, l'interdiction de toute intervention sur le PC et la distinction `main`/alpha restent en vigueur.

Voir [la dette](docs/engineering/RELIABILITY_DEBT.md), [l'issue #185](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/issues/185) et [la reprise](docs/engineering/GITHUB_HANDOFF.md). Les sections antérieures restent des archives datées.


## Point de reprise canonique — stabilisation vérifiée le 21 octobre 2026

**Observation avant cette mise à jour documentaire** — `main` = `83329d48789c20b1fc235d3096c3199dc9dc5176`; **aucune PR ouverte**. La [PR #297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) est fusionnée. Sa [CI indépendante post-fusion #39514832814](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39514832814) a réussi **dès la première tentative**, sur le commit exact : Rust Windows natif, Rust Linux et politique des dépendances. La préparation native sur la tête source exacte a également réussi. Vérifier de nouveau l'état GitHub : ce point de contrôle n'est pas une tête à réinitialiser.

**Livré sur main (preuves précises dans les PR, ne pas répéter ces incréments)** :

| PR | Fusion (SHA abrégé) | Résultat réellement intégré |
| --- | --- | --- |
| [#293](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/293) | `de973989` | Prévalidation coordonnée des journaux et fichiers de staging avant nettoyage des preuves temporaires |
| [#294](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/294) | `da3ab6f1` | Marqueurs de temps MCP du processus enfant, strictement filtrés dans le diagnostic Windows |
| [#295](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/295) | `615e6219` | Mesures distinctes des étapes Git, session, lecture et leases de processus |
| [#296](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/296) | `72966f92` | Mesures opt-in résolution de chemin / ouverture épinglée / lecture et hachage d'exécutable |
| [#297](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/297) | `83329d48` | Refus doublon/capacité **avant** la capture d'identité ; seconde vérification sous verrou avant insertion, tests négatifs et concurrents Windows/Linux |

La CI indépendante sur `main` a réussi dès la première tentative pour #293, #295, #296 et #297. **Pour #294, la CI post-fusion n'a réussi qu'à la deuxième tentative**, après une répétition du timeout natif Node/AppContainer de 10 secondes sur Windows ; l'échec de première tentative reste une preuve, pas un résultat à masquer.

**Mesures et portée exacte** : sur une exécution Windows de #296, `path_ms=0`, `pin_ms=0`, `hash_ms=4869` (temps de **lecture bornée et BLAKE3**, pas le calcul BLAKE3 isolé). Le correctif #297 évite ce travail coûteux **uniquement si l'inscription du lease est déjà irrecevable** ; il ne supprime pas et ne met pas en cache la vérification d'intégrité d'une inscription valide.

**Risques ouverts et indépendants** : intermittence Windows Node/AppContainer (~10 s sans premier marqueur du script malgré retour rapide du lancement), intermittence MCP de première réponse (~30 s historiquement), nettoyage multi-fichiers non atomique et risques d'identité des répertoires ancêtres / persistance sous coupure électrique. Les preuves Cargo préparées n'autorisent pas un Cargo général ; Electron avec sa sandbox normale reste non validé. L'alpha repose encore sur des protections/authentifications non considérées comme prêtes pour la production. Le projet reste **pré-alpha, sans release de production**.

**Directive de l'opérateur en vigueur** : gel des nouvelles fonctionnalités, Electron, Cargo produit, interface graphique, fusion globale de l'alpha et refactorings non justifiés. Corriger seulement un défaut démontré, avec tests ciblés ; ne pas fabriquer une PR lorsqu'aucun défaut n'est trouvé. Aucun build/test/installateur/permission sur le PC de l'opérateur, aucun runner auto-hébergé ni release, aucune suppression massive ; GitHub et ses runners hébergés uniquement. Préserver la sécurité des identités épinglées, leases/permissions, budgets, préconditions et preuves de récupération.

Voir [le relais des agents](docs/engineering/GITHUB_HANDOFF.md), [la dette de fiabilité](docs/engineering/RELIABILITY_DEBT.md) et [le prompt de reprise](PROJECT_CONTINUITY_PROMPT.md).

## Historical checkpoints — dated evidence superseded by the canonical section above

## Canonical verified stabilization checkpoint — 2026-10-20

At live inspection, `main` is `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`, **zero open PRs**, and the independent post-merge [CI #39394389914](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39394389914) completed successfully **on the exact main SHA, first attempt**, with hosted Windows Rust, Linux Rust and dependency-policy jobs all green.

- [PR #290](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/290) is **merged**, at `0c2a125537d020e3a3aff4124d12745d7a8c5c4c`. Its Windows Node/AppContainer test now records monotonic launch/terminal timings and child script-entry markers. Exact-source [CI #39392262806](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392262806) and independent [post-merge #39392829136](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39392829136) passed. This **does not resolve** the intermittent 10-second Node timeout or distinct 30-second MCP first-response timeout.
- [PR #291](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/291) is **merged**, at `f7daa7fc51f3e6058b442f9b948b3f7b2dea0a02`. A Windows recovery scan now validates **all** existing staging artifacts before deleting any, then revalidates each through the existing exclusive native delete handle. Exact-source [CI #39393849343](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849343), [native preparation #39393849287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39393849287) and post-merge #39394389914 passed. The negative Windows regression `corrupt_later_staging_does_not_erase_earlier_verified_staging` passed. The second deletion pass is **not multi-file atomic**; ancestor namespace races and sudden-power-loss persistence are not proved.

**Operator delivery policy remains stabilization-only:** no new functionality, broad refactor, experimental alpha merge, weakened CI, permissions or timeouts. Continue only with evidenced defects, targeted tests, and accurate documentation. No tests, builds or permissions changes on the operator's PC, no self-hosted runners, release or mass deletion. Every historical checkpoint below is dated evidence, **not** a live head or pending assignment; verify GitHub before further changes.

## Stabilization-only baseline — 2026-10-20

**Temporary delivery policy (operator decision, 2026-10-20): stabilization only.** Pause new features, product expansion, experimental merges, architectural reshuffles and broad refactors. Work exclusively on verified defects, security and recovery boundaries, flaky-test root causes, focused regressions, and accurate status/debt documents. Do not create code changes merely to keep PRs moving; when no defect can be demonstrated, record the evidence and stop. Preserve all existing permissions, isolation, deadlines, budgets and CI gates. Resuming feature development requires fresh operator direction.

Verified GitHub `main` at this checkpoint: `b1edc83438571867bd164ea0b2b9653304eed752`. [PR #288](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/288) was squash-merged at `b1edc83438571867bd164ea0b2b9653304eed752` from source `60206dcadbf2efb57800ca93f0bd226acf16f6fb`. Its hosted [CI #39388280621](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280621) passed Windows Rust, Linux Rust and dependency policy; [native preparation #39388280521](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388280521) passed too. Independent [post-merge CI #39388749406](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39388749406) succeeded on the same merge SHA, first attempt (Windows, Linux, dependencies). GitHub showed zero open PRs at verification.

**Delivered behavior:** `MutationRecoveryJournal::inspect_pending` validates all stable journal records before deleting any well-formed unpublished `*.prepared.tmp` files. A Windows/Linux regression preserves both the temporary and a later corrupt stable record on a failed scan, then checks normal cleanup after resolving corruption. Remaining boundaries include non-atomic multi-temp deletion, parent namespace/identity races, Windows directory persistence and unexplained Windows Node/AppContainer and MCP-first-response intermittent timeouts. These are risks for targeted investigation, not permission to claim fixes without proof.

## Archived proposal checkpoint — PR #288 (now integrated)

The preceding candidate wording was written before PR #288 passed native CI and merged. Its delivery is confirmed in the stabilization baseline above; do not implement it again.

## Linux journal directory sync integrated — PR #287 (2026-10-20)

[PR #287](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/287) was squash-merged as `0ca2305e7f01d5263330b67c4a0bb0520d452c8a` from final head `66161c9dee9e1a565b074d1cf105cf8961a712c2`. Hosted [PR CI #39386566516](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566516) and [native preparation #39386566570](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39386566570) passed; independent [main CI #39387133506](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39387133506) passed Windows Rust, Linux Rust and dependency policy on its first attempt. Linux GNU syncs the open recovery directory after publishing the stable prepared journal; failed sync retains a published record for recovery. Windows durability and sudden-power-loss guarantees remain distinct and unproven.

## Atomic no-replace journal publication — integrated by #286 (2026-10-20)

[PR #286](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/286) was squash-merged as `67c65a5d78a2921cee826184a2ad8bc573e4fb88` (exact head `7007ad3d202e42f642cece5f8df131035806ff35`). Hosted Windows/Linux Rust, dependency policy and the three native checks passed on that head; independent [post-merge CI #39385303625](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39385303625) passed Windows/Linux/dependencies on first attempt. The stable journal publication uses native Windows `MoveFileExW` without replace/copy flags and Linux GNU `renameat2(RENAME_NOREPLACE)`; competing publishers and existing destination evidence tests passed. Other OS targets fail closed. Directory pinning and complete power-loss durability remain separate.

## State-root creation preflight — pending focused PR (2026-10-20)

GitHub main `adeed077215c2103991254e4b5e2c64fa2d632de` and its [post-merge CI #39381445867](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39381445867) were checked: hosted Windows Rust, Linux Rust, and dependency policy succeeded on first attempt, with no open PRs before this increment. A separate targeted branch `fix/recovery-state-root-preflight` addresses a concrete setup side effect: `MutationRecoveryJournal::open` previously called `create_dir_all` before rejecting a requested state path inside the workspace. This could create nested directories in the workspace despite a rejected state-root request. The candidate first canonicalizes the closest existing ancestor and fails closed on an in-workspace ancestor *before* creating a directory, while retaining the post-create canonical check. Tests cover both rejected absent in-workspace state roots without side effects and accepted absent external state roots. This is **not integrated until exact-head hosted CI and merge**. Concurrent ancestor substitutions and symlink races remain outside this limited check; no general directory pinning or power-loss guarantee is claimed.


## Windows recovery staging cleanup — scoped source correction, 2026-10-20

At resumed inspection, `main` was `d11d27c8efb1048f504d96024e4015ec60fd1535`, with no open PRs and post-merge hosted [CI #39376986248](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376986248) passing all Windows Rust, Linux Rust and dependency checks on attempt 1.

[PR #284](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) isolates a Windows-only recovery staging-cleanup defect: the original code read/validated bytes, closed that file and unlinked a separately resolved name. Valid matching bytes in a staging hard link to an outside file were accepted without detecting the second link. The change reuses the existing exclusive no-reparse, single-link native delete handle, verifies the opened length and hash, and deletes via that **same handle**. A negative native Windows test requires both hard links and the journal evidence to survive failed recovery. This paragraph describes PR scope, not its merge/CI outcome; inspect the [PR's latest head/checks](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/284) before claiming delivery. Parent-directory pinning, concurrent ancestor swaps and power-loss guarantees remain unproven. The experimental alpha and Windows Node intermittency are unchanged.


## Recovery journal append alias hardening integrated — 2026-10-20

[PR #282](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/282) was squash-merged into `main` as `bf79b320b4e7aa04a02c87fe448d589296bc8bb2` from final source head `c837cead816fb59c5d3a94b8d9b5fe1926987644`. [CI #39375905332](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905332) passed hosted Windows Rust (including native AppContainer and MCP smokes), Linux Rust and dependency policy; hosted [native preparation #39375905331](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39375905331) also passed. The original head failed Clippy Windows due to an unnecessary `mut`, corrected without waiving checks. No PRs remained open after merging. The distinct [post-merge main CI #39376336498](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39376336498) was in progress at documentation time; check its actual final result.

Journal `append` no longer writes to hard-linked external evidence: Windows uses the existing exclusive no-reparse single-link open handle, seeks to EOF and syncs; Unix validates opened device/inode/link count against pre-open metadata. The size ceiling is rechecked on the opened file, and a cross-platform regression preserves evidence bytes when the file has an external hard link. No journal schema, authority or state transition changes. **This is not a solution** to Unix concurrent namespace/size races, Windows parent-directory pinning, staged artifact cleanup or general power-loss durability.

Windows Node/AppContainer 10s loopback and MCP first-response 30s intermittencies remain unresolved despite successful latest CI checks. Earlier failed attempts and exact-head retries remain evidence, not accepted causal fixes.

## Windows MCP startup timing evidence integrated — 2026-10-20

Fresh GitHub verification: `main` was `0f857b81a0715b2fcb7cb8f1e2a4250bf9221fa6`, with no open PRs; post-merge [CI 39370890039](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39370890039) passed on its second attempt. Its first attempt still contains a failed 30-second TEMP-only Node MCP initialization smoke; the separate AppContainer Node loopback 10-second intermittency also remains unresolved. These are not proven journal regressions.

[PR #280](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/280) was squash-merged as `8b6ccdc2270f8fa130df5d182d728d07466df908` from exact head `ce15d036332d314e7f951042285d4f442d0219a7`, which passed [hosted CI 39373512931](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373512931) (Rust Windows, Rust Linux and dependency policy). The separate post-merge [main CI 39373992973](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39373992973) was still in progress at this checkpoint. The scoped change provides opt-in monotonic startup phase timestamps on the bridge's stderr and host-side process-start/initialize timings for the existing GitHub-hosted Windows TEMP-only doctor. Static phase labels contain no path, credential, permissions or MCP payload. Ordinary bridge launches are unchanged unless `OPTIC_STARTUP_TIMING=1` is explicitly selected by the host. The diagnostic does **not** change 10s/30s timeouts, grant/isolation rules, CI acceptance conditions or the Rust core's optional language dependencies. The Windows TEMP-only doctor and Rust opt-in predicate test passed on the exact PR head. Its host-side timings on one passing Windows run were process-started=7ms, initialize-sent=30ms, initialize-received=4823ms (each from the doctor stopwatch). A successful diagnostic run does **not** prove or repair the cause of earlier timeouts.


## Recovery journal read and retirement hardening integrated — 2026-10-20

- [PR #277](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/277)
  was squash-merged into `main` as
  `c2282ff5eba85d2a7480dd353db638a0623bd06d`.
  Its final source head `825bcdb963d80ac8419df887e8b8c307aa497caf`
  passed hosted Windows Rust, Linux Rust, dependency policy
  ([CI 39368379416](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379416))
  and native preparation
  ([run 39368379422](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368379422)).
  An earlier head failed rustfmt and was corrected; canceled intermediate runs
  are not counted as passing. The independent post-merge `main` CI
  [39368741554](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39368741554)
  completed successfully on exact post-merge `main` (Windows Rust, Linux Rust,
  dependency policy).
- Read hardening rejects hard-link journal aliases on both hosted platforms:
  Windows uses the existing exclusive no-reparse, single-link read primitive;
  Unix validates the opened descriptor's identity and link count against
  pathname metadata. This is not a complete race/power-loss guarantee.
- [PR #278](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/278)
  squash-merged as `b15fe7636dcb31033d71ecdfea9ed51bdc786a2b`
  from exact final head `9778b497320d82db493bfbe4c02ff13d113c8ad5`.
  Hosted [CI 39369331802](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331802)
  passed Windows Rust, Linux Rust and dependency policy; hosted
  [native preparation 39369331816](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369331816)
  passed. An earlier PR head failed rustfmt and was corrected, not waived.
  Windows journal retirement now uses an exclusive validated handle for
  deletion. Unix rejects aliased evidence before the existing pathname unlink;
  a concurrent pathname replacement remains possible. The separate post-merge
  `main` [CI 39369706540](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39369706540)
  initially failed its Windows Rust test on the **known intermittent**
  `capability_free_appcontainer_denies_node_loopback_connection`: owned Node
  returned `TimedOut` after 10 seconds with no output. Three immediate
  serial diagnostics passed. The original first attempt is preserved and
  the failed-Windows-job-only second attempt on the identical `main` SHA
  **passed** all applicable Windows tests and smokes. Linux/dependency jobs
  passed on the original attempt. CI #39369706540 is therefore completed
  successfully on attempt 2, with the original failure retained as evidence.
  There were zero open PRs immediately after the merge.
- Next priority after this recurrent failure: isolate native AppContainer/Node
  startup and scheduling with bounded diagnostics **without** increasing the
  timeout or disabling the network-isolation proof. A separate future domain
  review is journal appends and their opened-file identity, followed by staged
  artifact cleanup or directory pinning, with demonstrable defects and tests.
  Append, staging cleanup, power-loss durability and Unix pathname deletion
  races are not covered by #277/#278. Experimental alpha remains separate.
  No operator-PC or self-hosted execution occurred.

## Final GitHub-verified continuation checkpoint — 2026-10-20

- At this checkpoint, `main` = `e222ff912ac784eb7ae29d09e74ae1780f8a051b` (squash merge
  [PR #275](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/275)).
  GitHub reported **zero open pull requests**.
- The exact post-merge `main` CI
  [run 39364949397](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364949397)
  completed successfully: GitHub-hosted Windows Rust, Linux Rust and dependency
  policy all succeeded. This confirms the state of that commit, **not** an
  unconditional guarantee for future branch heads.
- Post-merge `main` CI for #274
  [run 39364345041](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39364345041)
  also completed successfully. #273 post-merge run 39363288153 succeeded.
- Source/test outcome remains narrowly scoped: six unchanged Rust source files
  reorganized under `src/mutation/`; the recovery parser now rejects complete
  blank journal records, retaining evidence while tolerating torn final lines.
  The first-attempt AppContainer/Node loopback timeout on #274 remains a separate
  unresolved intermittent failure despite a successful exact-head rerun.
- `alpha/generated-project-editing` remains a separate experimental branch;
  no full alpha-to-main merge, release, branch bulk-deletion, local operator-PC
  test, installer, tunnel or self-hosted run occurred.

**Next bounded review:** mutation recovery journal open/read/retire boundaries
and evidence retention, or a dedicated investigation of Windows Node/AppContainer
cold-start timing. Read [GITHUB_HANDOFF](docs/engineering/GITHUB_HANDOFF.md)
and [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md) before changing code.

## Mutation and recovery stabilization — 2026-10-20

- [PR #273](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/273)
  merged into `main` as `58ebc5c1a4f0fd015b26c04f8cd83cf4255d5782`.
  Six unchanged Rust source blobs were relocated into `src/mutation/`;
  their original crate-root module names, public exports and inline tests
  remain unchanged. Exact source head `9758492` passed all four applicable
  hosted Windows/Linux Rust, dependency and native-preparation checks;
  post-merge `main` CI 39363288153 passed.
- [PR #274](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/274)
  merged as `58a72a202bbdb57da425fba461a7b1a97b2f9d92`.
  The recovery journal parser now fails closed on a complete empty journal
  record while still allowing a nonterminated torn final record. Regression
  tests cover interior/trailing blank records, retained journal evidence and
  torn-tail classification. The first source head `8d51d19` failed rustfmt
  and was corrected. Exact final head `283c9e0` passed Linux, dependency
  policy and native preparation; Windows test failure on the first CI
  attempt was an AppContainer/Node loopback probe timeout, **not** a parser
  failure. Three serialized diagnostics passed on the same source, and a
  failed-Windows-job-only rerun passed the entire applicable Windows job.
  Both attempts are retained in CI run 39363540900. The cause of that
  intermittent native test remains unknown and is tracked in
  [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).
  Post-merge `main` CI run 39364345041 subsequently completed successfully
  on hosted Windows/Linux and dependency policy.

No operator-PC execution, install, permission change, tunnel or self-hosted
runner. These changes do not establish general Cargo/Electron support,
durable disk quotas, power-loss ACID recovery or a complete source audit.

## Latest reviewed stabilization — 2026-10-20

- [PR #270](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/270)
  merged as `636a8b5e917fd731dbd8566e118958f141f67542`.
  Four session-owned modules were moved byte-identically into
  `crates/optic-bridge-runtime/src/session/`. Crate-root module names,
  test namespaces and public exports did not change. Final source head
  `4d5ff0a` passed GitHub-hosted Windows/Linux Rust, dependency policy
  and native preparation checks; post-merge main CI run 39360825623 passed.
- [PR #271](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/271)
  merged as `8610c8dc2558695842412c5a0fce993d2420d6fb`.
  Unexpected stdout/stderr Tokio drain task join failures are now treated
  as incomplete output, not silently discarded. Two synthetic regressions
  cover task panic and intentionally aborted drains after
  `TerminationUncertain`. Final source head `fb9ffdb` passed four hosted
  Windows/Linux Rust, dependency and native preparation checks. The initial
  `bed3ca9` head failed rustfmt and was corrected. Separate post-merge
  main CI run 39361585398 passed.
  
No tests/builds/installers/permissions/tunnels were executed on the operator's
PC; no self-hosted runners. These increments do not prove normal Cargo,
sandboxed Electron, general disk quotas, universal process logging durability
or completion of an exhaustive source audit.

## Offline Cargo research integration — 2026-10-20

PR255 is merged as `f1e72f8`. It completes the fixed research fixture and optional
preparation design, not general Cargo or administrator setup. At final head
`6bbcb64`, research [run 39339860948](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39339860948)
passed prepared job118358440446, including unrelated-profile denial, known-output
cleanup and explicit revoke. Independent normal job118358440142 retained NUL
error5 / exit101. Standard direct-Rust CI is unchanged. See the
[research owner](docs/research/PRIVATE_OFFLINE_CARGO_NUL_2026-10-19.md).


Reviewed: 2026-10-20. This page describes source and measured acceptance, not a
production release. Detailed historical notes are retained in
[the exact previous status snapshot](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/blob/e1b82888dcf9c079740777be27341ca943758859/STATUS.md).

Current continuation is [GitHub-only](docs/engineering/GITHUB_HANDOFF.md): no
operator-PC testing or runtime changes. The [new-chat prompt](PROJECT_CONTINUITY_PROMPT.md)
and [agent entry point](AGENTS.md) do not require old chat memory or local files.

## Current structural delivery and Windows smoke evidence

[PR #268](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/268)
merged to `main` as `d79d60b88b60b2e95c85039c2d31427d0954a865`.
The ten prior `src/git_*.rs` modules are now under `src/git/`, with
identical content hashes and unchanged crate-root module names and
public exports. Exact PR head `b2b5207` passed four hosted checks:
Windows/Linux Rust, native preparation and dependency policy. This is a
mechanical organization improvement, not new behavior or an audit of all files.
Post-merge `main` CI run 39358651176 passed; the subsequent #269
post-merge main CI 39359157209 also passed.

Earlier [run 39356540735](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39356540735)
on `main` initially failed the Windows TEMP-only Node MCP doctor smoke
after a 30-second response timeout with the bridge still alive. Re-running
only the failed Windows job at the exact same SHA succeeded, without a
source or workflow change. Root cause is not proven; the failure remains
recorded in [RELIABILITY_DEBT](docs/engineering/RELIABILITY_DEBT.md).

## Merged and validated

PR266 merged into main as `5a8157e`: a targeted process stdout/stderr
reader-error correction no longer treats a read failure as clean EOF.
A distinct private read-failure bit marks output incomplete, stops a running
owned job through the existing bounded confirmation path, and converts an
otherwise successful exit to `Failed` after pipe drains complete. Timed-out,
stopped and termination-uncertain jobs retain their specific status. Two
synthetic async-reader regressions were added. Final PR head `4e4c712`
passed hosted Windows/Linux Rust, dependency policy and native preparation
checks; the prior revision failed rustfmt and was corrected. This is not a
general guarantee that every process log is durable or fully collected.
[PR #266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/266).
The separate main push CI run 39356012838 was still executing at the
documentation checkpoint; check its live result.

PR265 merged into main as `a9d9a31`: the `process.rs` test body
was moved unchanged into `process/tests.rs`; production logic remained
byte-identical, with 27 tests preserved. Exact head `f65f69b` passed four
hosted Windows/Linux Rust, dependency-policy and native-preparation checks.
This is structural code quality, not a runtime security fix. The separate
process-output diagnostic issue and its regression work are tracked by
[CODE_QUALITY_REVIEW](docs/engineering/CODE_QUALITY_REVIEW.md); an unmerged
fix is not counted as shipped behavior.

PR263 merged into main as `bcb5e8a` after exact head `2d8a410` passed four applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation). It extracts unchanged inline project-input test logic into a child test module and introduces the scoped [code quality register](docs/engineering/CODE_QUALITY_REVIEW.md); it is **not** an exhaustive file audit or a behavioral improvement. [PR #263](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/263).

PR262 merged into main as `bca2eae` after its exact head `14b978b` passed all six applicable hosted checks (Windows/Linux Rust, dependency policy, native preparation, lifecycle and direct Rust). It adds a host-only, version-pinned retirement API for strictly closed private task evidence, with no model-visible deletion, automatic replay or operator workflow. This does **not** solve general retention UX or eliminate unresolved recovery capacity. [PR #262](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/pull/262). No operator-PC execution. A separate source-organization review is tracked in [the quality register](docs/engineering/CODE_QUALITY_REVIEW.md); extracted tests require their own exact-head CI proof.


PR259/`97f4098` adds lossless Windows path boundary checks and a native
regression for a reproduced alias read across distinct non-Unicode roots.
Git path comparisons/arguments and operator-policy placement receive the same
representation audit. See [ADR-0033](docs/decisions/ADR-0033-LOSSLESS-NATIVE-PATH-BOUNDARIES.md)
and the [reviewed debt register](docs/engineering/RELIABILITY_DEBT.md); this is not
a claim that every filesystem or compatibility gate is closed. Exact
CI39338661674 and preparation39338661374 passed before merge.

| Track | Latest delivered increment | Evidence |
| --- | --- | --- |
| Main core | Native path hardening #259; fixed offline Cargo research #255 | #259 merged `97f4098`; #255 head `6bbcb64` passed CI 39339859057, preparation 39339859114, pipes 39339859071 and direct Rust 39339859055; merged `f1e72f8`. Research normal failure remains separate. |
| Generated FS alpha | Completed Electron diagnosis #250; hardened-core ancestry alignment #260 | #260 head `a4091bc` passed CI 39340735293, adapter/package 39340735266, preparation 39340735187, pipes 39340735200 and direct Rust 39340735265; merged `5245e3d`. Cargo/Electron research baselines remain red. |
| Previously verified alpha package | Source CI merge `ac76efd6ea833e46dfced7bee3148af968754566`, artifact 12229445645 | 85 archive entries and 80 source files verified; native packaged 32 KiB HTTP editing and Node fail/fix/check/test passed before the no-PC continuation. |

The alpha ZIP SHA256 is
`3563bb8e9666455e6559cf6e30362b99849eb437fe91769fc1b6691c2113bc00`.
Its source identity remains fixed; a documentation change does not relabel it.
It uses generated projects, shared connection authority and public dummy auth.
The older `b2b34a47…` package predates #259 and is historical. Actions artifacts
can expire; inspect availability in [run 39340735266](https://github.com/ELITEAYOTO/Optic-AI-Bridge-/actions/runs/39340735266)
instead of assuming a local ZIP is accessible to a new agent. No production
release or new PC test is implied by this provenance update.

The core includes scoped/versioned filesystem mutation, separately authorized
Git operations, bounded process lifecycle and private-service foundations.
Not every internal service is exposed as a general MCP project/build tool.

## Compatibility evidence and remaining limits

- **Electron #250:** bounded diagnostic scope merged into alpha as `e62ea95`.
  At head `62f8177`, ordinary CI/adapter/preparation/pipes/Rust passed. The
  staging rerun job118357480984 passed both diagnostic controls and observed
  normal exit `0x80000003` / primary result11 before app startup. The separately selected outer-only diagnostic succeeds; no automatic
  sandbox downgrade or public GUI permission follows. The baseline research
  staging workflow remains red, including run39340735239 on #260; its controls
  and separately selected outer-only comparison do not change normal support.
- **Cargo #255:** ordinary capability-free execution fails opening NUL, exit 101.
  At source `21ac09b`, disposable CI 39291321987/job 118192403768 passed the prepared
  exact-profile comparison, actual checks/tests, output cleanup and explicit
  revoke with restored denial. This is not ordinary Windows Cargo acceptance.
- **Optional Windows preparation:** development direction approved; the proposed
  contract is [in main](docs/research/OPTIONAL_WINDOWS_NUL_PREPARATION.md). No local privileged helper or device permission change
  has been applied. Authentication, lease provenance and interrupted recovery
  remain implementation gates.
- **Real projects:** writable output containment, dependency restore, secrets,
  enrollment and artifact publication require their own proven contracts.
- **FS failures:** retained local evidence cannot expose FS backend logs or
  restore hidden model state; unknown writes must be observed before retry.

## Repository consolidation

41 historical PRs were closed with explicit disposition. Their work remains in
alpha ancestry, exact GitHub pull refs or published archive tags. The later #250
and #255 integration completes their bounded measurement scopes; unresolved
compatibility work remains in this status and the debt register, rather than
indefinitely open diagnostic carriers.

244 historical remote branches were removed after explicit operator approval,
exact-head/preservation revalidation and backup hash verification. The four
reference branches and all preservation refs were verified unchanged at that
retirement checkpoint. After later integrations #250/#255/#260, the pre-handoff
inventory was zero open PRs and two branches: `main` and
`alpha/generated-project-editing`; completed short-lived branches were retired.
This dated inventory must be refreshed live by the next agent. Exact historical
heads, approval and observed results are in the
[consolidation record](docs/engineering/REPOSITORY_CONSOLIDATION_2026-10-20.md).

Next delivery steps are owned by [ROADMAP.md](ROADMAP.md). Core and alpha remain
separate until their applicable release gates are satisfied.
