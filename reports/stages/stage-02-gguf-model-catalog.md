# Stage 02 — GGUF model catalog and identity

**Date:** 2026-10-21
**Maintainer:** `r31nu11` (project maintainer)
**Branch:** `dev`
**Status:** Complete, pending formal acceptance
**Content commit:** the Stage 02 content commit that introduces this report; the
follow-up report/status commit records its exact hash.

## 1. Stage identifier and title

Stage 02 is **“GGUF model catalog and identity”** from
[the roadmap](../../docs/ROADMAP.md). It was explicitly authorized after Stage 01
acceptance. This report records the implementation, evidence, limitations, and
review state; it does not accept the stage or start Stage 03.

## 2. Objectives and acceptance criteria

| Objective or criterion | State | Evidence |
| --- | --- | --- |
| Add exactly one workspace member, `ghostram-models`, reusing `ghostram-core` and extending the existing CLI. | Met | The workspace lists four members and `Cargo.lock`; no other crate was added. |
| Parse untrusted GGUF v2/v3 little-endian files with bounded reads, checked arithmetic, explicit limits, typed errors, and no tensor-payload parsing. | Met for synthetic files | `crates/ghostram-models` parser tests, all prefixes of a valid header, malformed/adversarial fixtures, and the documented limits. |
| Reject unsupported versions, endianness, type codes, duplicate metadata keys and tensor names, invalid alignment, truncation, and limit violations explicitly. | Met | Parser tests cover each category and stable failure kinds; the format boundary is documented. |
| Extract compact metadata only; never emit vocabularies, merges, chat templates, prompts, or arbitrary blobs. | Met | Extracted fields are enumerated in the schema; the JSON contract test asserts token values, merge values, and template text are absent. |
| Implement `ghostram models scan <path>` with human/`--json` output, non-recursive default, `--recursive`, and explicit `--hash`. | Met | CLI unit and integration tests cover help, default behavior, recursion, hashing, JSON, human output, and exit codes. |
| Scan only user-selected directories; never follow symlinks, invent defaults, cache results, write to models, or add network/database/persistence. | Met | Root and symlink tests, scan design, and code inspection; the scan envelope is ephemeral. |
| Define a typed `ghostram.models.scan/1` schema and leave `ghostram.doctor/1` unchanged. | Met | `docs/MODELS_SCHEMA.md`; doctor schema and diagnostic behavior are unchanged. |
| Stream optional SHA-256 with bounded memory; detect changes and never report a partial digest as complete. | Met for synthetic files | Hash unit tests and scan change tests; fixed 64 KiB buffer; digest discarded on failure or change. |
| Add deterministic synthetic/adversarial tests; no GPU, engine, model download, network, root, inference, or benchmark. | Met | 63 model-crate tests and 15 CLI integration tests; no privileged or network test was added or run. |
| Update documentation, changelog, policy, status, roadmap, and this 14-section report. | Met | The documentation list in section 13; documentation check passes. |
| Run all Stage 02 validation checks and record exact commands and outcomes. | Met | Section 7; final committed results are recorded there. |
| Commit to `dev` with the maintainer identity and publish to `origin/dev` without force or rewriting. | Met for commits; publication is a post-report step | Section 11 records the content commit and final-HEAD convention. The push is an ordinary fast-forward after this commit. |

A real-model cross-check was **not** an acceptance criterion for this authorized
Stage 02 scope. It was deferred and is not claimed.

## 3. Summary of completed and incomplete work

Completed:

- Added `crates/ghostram-models` with focused modules for errors, catalog types,
  filename hints, streaming SHA-256, bounded GGUF parsing, traversal/scanning, and
  test support.
- Added the bounded GGUF reader for v2/v3 little-endian headers, all documented
  metadata value types, nested arrays within a limit, metadata keys, tensor
  descriptors, alignment, and tensor-type identifiers.
- Added compact extraction of general fields, common architecture fields,
  tokenizer summaries and presence flags, tensor types, alignment, tensor-data
  region facts, and warnings. Vocabulary values, merges, added-token values,
  templates, and arbitrary blobs are skipped.
- Added one-root directory scanning with sorted, non-recursive default traversal;
  opt-in recursion; symlink rejection; non-UTF-8 and unreadable diagnostics;
  entry/depth/issue limits; filename hints; and per-file outcomes.
- Added optional SHA-256 using a fixed 64 KiB streaming buffer and complete-read
  checks, plus race-aware change detection that discards a digest and parsed
  metadata on an observed change.
- Added `ghostram models scan` and `--json`, human rendering with terminal-control
  escaping, stable exit codes, and the typed `ghostram.models.scan/1` contract.
- Moved shared UTC RFC 3339 formatting into `ghostram-core::time` so the probe and
  model paths use the same formatter.
- Added 63 model-crate unit tests and 15 CLI integration tests, including
  deterministic synthetic GGUF files and adversarial cases.
- Added and updated all documentation listed in section 13.

Not completed, and not silently claimed:

- No real GGUF model was available or tested. No metadata was cross-checked against
  an engine because no engine is installed.
- No shard grouping or reconstruction, `models list`, cache, database, watch mode,
  default model path, or scan-to-scan comparison was added. This scope was
  superseded by maintainer authorization and reconciled in the docs.
- No stable `ModelId` was implemented. Optional hashes and paths are observations;
  identity rules remain a later decision.
- Big-endian GGUF and v1 are unsupported.
- Tensor payload length, quantization block, overlap, and inference compatibility
  checks were not implemented.
- No release, tag, installer, `main` promotion, inference, benchmark, GPU compute,
  or hardware measurement was performed.

## 4. Architectural decisions and rationale

No new ADR was required. The fourth crate follows the modular-monolith and
crate-split rules of
[ADR-0002](../../docs/adr/0002-modular-monolith-and-crate-split.md), and no accepted
decision is rewritten or superseded.

Key decisions:

- **Parser boundary:** accept GGUF v2/v3 little-endian only. This covers the
  current little-endian files worth cataloging, keeps the parser auditable, and
  avoids pretending to support big-endian payload addressing or v1 structure.
  Detected big-endian v2/v3 version words are classified
  `unsupported_endianness`.
- **Unknown metadata versus unknown tensor types:** undocumented metadata type
  codes are unsupported because the reader cannot safely know their shape. Unknown
  tensor type identifiers are retained as numeric catalog facts with no invented
  name and an `unknown_tensor_type` warning, because the file can remain
  structurally parseable.
- **Alignment:** follow the GGUF specification's `uint32`, nonzero, multiple-of-8
  rule. A present non-`uint32` value is fatal `invalid_alignment`. The upstream
  loader's stricter power-of-two check and its effective `<64` tensor-name boundary
  are documented differences, not silent conformance claims.
- **Key grammar:** enforce key length, UTF-8, non-empty, non-duplicate, and bounded
  totals, but not the specification's full ASCII lower-snake hierarchy grammar.
  Only known keys are interpreted; accepting an unknown safe spelling avoids
  rejecting a file for a key the catalog does not use. This relaxation is recorded
  in the research note.
- **Resource limits:** use conservative project limits for metadata entries, key
  bytes, strings, arrays, nesting, tensors, names, dimensions, distinct types,
  traversal, issues, and the hash buffer. A file above a product limit is
  `unsupported`; the limits are not claims about GGUF itself.
- **Tokenizer handling:** treat `tokenizer.ggml.added_tokens` as `array[string]`, as
  confirmed upstream. Retain only array element type and count.
- **Change semantics:** if identity, size, or modification time changes, or hashed
  bytes differ from the starting size, classify the file `changed`, discard any
  parsed metadata and digest, and preserve the original failure if one existed.
- **Schema:** introduce `ghostram.models.scan/1` for model observations and keep
  `ghostram.doctor/1` unchanged. The model contract is additive and versioned; it is
  not a stable model-identity or engine-compatibility contract.
- **Hashing:** no implicit file-wide read. `--hash` is an explicit, streaming,
  complete-file operation, because hashing a multi-gigabyte model can be expensive
  and materially different from reading descriptors.
- **Scope reconciliation:** cache, `models list`, shard grouping, configured default
  roots, mandatory hashing, and real-model cross-check from earlier roadmap wording
  are deferred and documented as superseded. Filename hints remain informational.

Format evidence is in the dated
[GGUF catalog format research](../../docs/research/2026-10-21-gguf-catalog-format.md).

## 5. Files added, changed, or removed

Added:

- `crates/ghostram-models/Cargo.toml`
- `crates/ghostram-models/src/lib.rs`
- `crates/ghostram-models/src/error.rs`
- `crates/ghostram-models/src/catalog.rs`
- `crates/ghostram-models/src/gguf.rs`
- `crates/ghostram-models/src/hash.rs`
- `crates/ghostram-models/src/filename.rs`
- `crates/ghostram-models/src/scan.rs`
- `crates/ghostram-models/src/test_support.rs`
- `crates/ghostram-core/src/time.rs`
- `docs/GGUF_CATALOG.md`
- `docs/MODELS_SCHEMA.md`
- `docs/research/2026-10-21-gguf-catalog-format.md`
- `reports/stages/stage-02-gguf-model-catalog.md`

Changed:

- `Cargo.toml`, `Cargo.lock`
- `crates/ghostram-cli/Cargo.toml`
- `crates/ghostram-cli/src/main.rs`
- `crates/ghostram-cli/src/render.rs`
- `crates/ghostram-cli/tests/cli.rs`
- `crates/ghostram-core/src/lib.rs`
- `crates/ghostram-probe/src/lib.rs`
- `crates/ghostram-probe/src/report.rs`
- `README.md`, `AGENTS.md`, `CHANGELOG.md`
- `docs/PROJECT_STATUS.md`, `docs/ROADMAP.md`, `docs/ARCHITECTURE.md`
- `docs/CONTRIBUTING.md`, `docs/TESTING.md`, `docs/ENGINE_SUPPORT.md`
- `docs/INSTALL.md`, `docs/QUICK_START.md`, `docs/TROUBLESHOOTING.md`
- `docs/UNINSTALL.md`, `docs/RELEASING.md`
- `scripts/check_docs.py`

Removed:

- `crates/ghostram-probe/src/time.rs`, moved to `ghostram-core::time` so both
  consumers share one UTC formatter. Its tests moved with it.

No user model, engine, benchmark, or unrelated file was modified or deleted.

## 6. Dependencies added or modified

- Added workspace dependency `sha2 = "0.10.9"` for SHA-256.
- Added `ghostram-models` to the workspace member list and as a path dependency of
  `ghostram-cli`.
- `ghostram-models` depends on `ghostram-core`, `serde`, and `sha2`; on Unix it uses
  the existing `libc` dependency for non-following candidate opens; dev-only
  `serde_json` supports contract tests.
- `ghostram-cli` now depends on `ghostram-models` and directly on `serde` for
  serialization adapters used by human output.
- `Cargo.lock` records `sha2` and its transitive crates: `digest`, `block-buffer`,
  `crypto-common`, `generic-array`, `typenum`, `cpufeatures`, `cfg-if`, and
  `version_check`.
- No engine, model, database, network, GUI, async runtime, or persistence
  dependency was added. No dependency version was yanked or replaced.

## 7. Validation commands and observed results

All commands below were run from the repository root on 2026-10-21 after the final
code edit. Long test output is summarized by the exact per-binary totals printed by
Cargo.

```text
$ cargo fmt --all -- --check
(no output; exit 0)

$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking ghostram-models v0.1.0-dev
    Checking ghostram-cli v0.1.0-dev
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.99s

$ cargo test --workspace --locked
test result: ok. 12 passed; 0 failed; 0 ignored (ghostram CLI unit)
test result: ok. 15 passed; 0 failed; 0 ignored (ghostram CLI integration)
test result: ok. 16 passed; 0 failed; 0 ignored (ghostram-core)
test result: ok. 63 passed; 0 failed; 0 ignored (ghostram-models)
test result: ok. 41 passed; 0 failed; 0 ignored (ghostram-probe)
Doc-tests: 0 passed each for core, models, and probe
Total: 147 passed, 0 failed

$ cargo build --workspace --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s

$ python3 scripts/check_docs.py
ok: 33 Markdown files, 163 relative links, 28 required files

$ bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD
(the range check is run over the completed Stage 02 history; the follow-up
report/status commit records the observed result)
```

Additional final checks:

```text
$ git config --local --get user.name
r31nu11
$ git config --local --get user.email
r31nu11@mail.ru
$ git diff --check
(no output; exit 0)
```

During implementation, the following issues were found and corrected before the
final validation:

- `tokenizer.ggml.added_tokens` had initially been treated as an integer array; it
  is `array[string]`. A parser fixture and regression test now pin the upstream
  type.
- A present non-`uint32` `general.alignment` could have fallen back to the default.
  It is now always fatal `invalid_alignment`, with float/string/bool/array tests.
- Presence-only chat-template and Hugging Face tokenizer flags are now retained
  when the key is present with an unexpected scalar type.
- Per-file change handling now discards parsed metadata as well as the digest. The
  change test asserts no metadata remains.
- Budget tests were added for total metadata-key/tensor-name bytes and the
  distinct-tensor-type limit.

No command failure was hidden. The final documentation check was run only after all
required files, including this report, existed.

## 8. Hardware tests and measurements

No GPU, inference, engine, model-load, benchmark, or hardware measurement test was
performed in Stage 02. No engine is installed, no real model was supplied or
downloaded, and no model cache exists. The test machine was not re-probed in this
stage; the last accepted environment snapshot remains the
[Stage 01 report](stage-01-minimal-rust-workspace.md).

The parser and scanner tests ran on the ordinary development host file system using
small synthetic files in temporary directories. Those are functional tests, not
throughput or memory measurements. No timing, token rate, VRAM use, disk
throughput, or hashing throughput number is reported.

## 9. Tests that could not be performed

| Test | Missing prerequisite | Risk of leaving it untested |
| --- | --- | --- |
| Parse and hash at least one real GGUF model. | No real model available; downloading a large model was not authorized. | Synthetic fixtures may miss real-world metadata ordering, tokenizer combinations, or large-file behavior. |
| Cross-check metadata against engine-reported model properties. | No engine installed and no inference launch authorized. | Catalog fields are not proven to match what an engine will report or accept. |
| Shard grouping and reconstruction. | Deferred scope; no multi-shard model available. | Users must keep shard records separate; an incomplete split model could be misread as multiple models. |
| Big-endian and v1 parsing. | Deliberately unsupported, not merely untested. | Users with those files receive `unsupported`, not metadata. |
| Tensor payload and quantization block validation. | Out of scope for a metadata catalog. | Structurally cataloged files may still be rejected by a loader, and offsets may not describe complete non-overlapping ranges. |
| Concurrent-writer race and network-filesystem blocking. | No controlled concurrent writer or network mount test harness. | Change detection remains best effort, and a blocking file system call has no timeout. |
| Windows and non-Linux scan behavior. | No Windows machine or CI runner executed. | Symlink behavior and non-following opens differ by platform; current implementation is validated only on Linux source builds. |
| Release artifact, installer, and end-user invocation. | No package stage or release approval. | Nothing about release behavior can be claimed from source tests. |

## 10. Known bugs, limitations, and unresolved questions

Known limitations, not hypotheses:

- No real-model evidence exists. Synthetic fixtures prove parser behavior and
  rejection paths, not compatibility with a specific model family.
- No stable model identity. A path is not an identity, and SHA-256 is optional. The
  same model can appear under multiple paths, and shards are not grouped.
- Big-endian and GGUF v1 are unsupported. The endianness heuristic covers big-endian
  v2/v3 version words, not every conceivable malformed byte order.
- Metadata key grammar is not fully enforced. Unsafe key spellings are not used for
  extraction, but the catalog accepts a broader key set than the specification.
- Tensor starts are checked for alignment and bounds, not complete non-overlapping
  ranges. Payload sizes, quantization block sizes, and contents are not validated.
- Change and hash consistency are best effort. A same-size, same-mtime concurrent
  modification can be missed; the scanner does not lock files.
- Non-UTF-8 candidate paths are not cataloged and make the result `partial`.
- File-system reads and seeks have no I/O timeout. A pathological regular file or
  mount can block the command.
- The command is source-only. There is no installer, release, or stable end-user
  path.
- No metrics, logs, cache, watch mode, or scan history exist.

Unresolved questions requiring maintainer or future-stage direction:

- What evidence threshold is required before the catalog is used for a launch
  decision? A real-model plus engine cross-check is still needed.
- Should a complete content hash become mandatory for stable identity, and how
  should split shards be grouped without trusting filenames?
- What compatibility behavior should be adopted if the upstream loader and the
  written GGUF specification disagree further around alignment or tensor names?
- Should blocked file-system reads eventually need a worker/timeout design?

No known memory-safety or panic bug was observed in the tested fixtures. That is not
a proof of absence for arbitrary inputs.

## 11. Current branch and commit hash

Branch: `dev`.

The Stage 02 content commit is the commit that introduces this report and the
implementation. Its exact hash is recorded by the follow-up report/status commit,
which is the current HEAD when this report is read in a clean tree; that HEAD hash
is visible with `git log -1` (this report cannot embed the hash of the commit that
contains this sentence). Both are ordinary non-merge commits authored as
`r31nu11 <r31nu11@mail.ru>`. No history was rebased, amended after publication, or
force-pushed.

At content freeze, `origin/dev` was at
`bdfced9d7db49a1e2bdad5dd1602822734da33b1`; `main` and `origin/main` were also at
that commit. Publication of the Stage 00/01/02 history to `origin/dev` was explicitly
authorized and is a fast-forward of the local `dev` branch. No tag, GitHub Release,
artifact, `main` promotion, or Stage 03 authorization is part of this delivery.

## 12. Git working-tree status

Before the content commit, the working tree contained only the Stage 02 additions,
modifications, and the intentional move of the probe UTC formatter. `git diff
--check` reported no whitespace errors. The final report/status commit contains only
this report and `docs/PROJECT_STATUS.md` changes. No generated model, engine, test
artifact, cache, secret, or unrelated user file was staged or committed.

The tree is expected clean after the final report/status commit; the final hand-off
verifies `git status --short` and reports the result. Untracked build output under
`target/` is ignored by the existing repository rules.

## 13. Documentation updated

- `README.md`: current stage, capabilities, source commands, safety, and doc index.
- `AGENTS.md`: four-crate map, Stage 02 state, non-claims, and validation commands.
- `CHANGELOG.md`: Stage 02 additions, corrections, security notes, and reconciliation.
- `docs/PROJECT_STATUS.md`: Stage 02 snapshot, validation, environment, deferrals.
- `docs/ROADMAP.md`: Stage 01 accepted; Stage 02 complete pending acceptance; Stage
  03 blocked; superseded scope recorded.
- `docs/ARCHITECTURE.md`: model crate boundary, model-scan contract, data flow, model
  identity status, and testing seam.
- `docs/TESTING.md`: Stage 02 tests, synthetic-fixture policy, and untested hardware.
- `docs/ENGINE_SUPPORT.md`: model format implementation, parser choice, engine
  cross-check gap, and GGUF sources.
- `docs/CONTRIBUTING.md`: current workflow status.
- `docs/INSTALL.md`, `docs/QUICK_START.md`, `docs/TROUBLESHOOTING.md`,
  `docs/UNINSTALL.md`: source-only commands, scan boundaries, errors, and absence of
  installed state.
- `docs/RELEASING.md`: publication authorization without a release or tag.
- `docs/GGUF_CATALOG.md` (new): parser/scan behavior, limits, hashing, privacy, and
  limitations.
- `docs/MODELS_SCHEMA.md` (new): complete `ghostram.models.scan/1` contract and
  example.
- `docs/research/2026-10-21-gguf-catalog-format.md` (new): dated upstream sources,
  confidence labels, conformance differences, and non-claims.
- `scripts/check_docs.py`: requires the new catalog, schema, research note, and stage
  report.

Documentation check: `ok: 33 Markdown files, 163 relative links, 28 required
files`.

## 14. Recommended next stage

Maintainer review and formal acceptance of Stage 02 are recommended next. If
accepted, Stage 03 (memory planner and safety budgets) may be separately authorized.
Stage 03 must not begin from this report alone.

If the maintainer wants stronger model evidence before acceptance, the next action
should be a separately approved real-model validation with an exact model hash,
engine build, command, and observed metadata comparison. This report does not claim
that validation exists. Tags, releases, and `main` promotion remain separate
decisions.
