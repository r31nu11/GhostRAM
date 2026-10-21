# Stage 03 — Memory planner and safety budgets

**Date:** 2026-10-21
**Maintainer:** `r31nu11` (project maintainer)
**Branch:** `dev`
**Status:** Complete, pending formal acceptance
**Content commit:** `2dda210a28f88bee4dc6c2538db9224ca1cdb5d6`
(`feat(planner): add deterministic memory planning`).
**Final report/status commit:** the descendant `docs(stage-03)` commit; use
`git log -1` at hand-off.

## 1. Stage identifier and title

Stage 03 is **“Memory planner and safety budgets”** from
[the roadmap](../../docs/ROADMAP.md). It was authorized after the maintainer
accepted Stage 02 on 2026-10-21. The delivered scope is a deterministic, pure
memory planner for one explicitly selected GGUF file, not an inference optimizer or
runtime governor.

This report records the implementation, formula evidence, tests, documentation,
limitations, and local validation. It does not accept the stage. Stage 04 remains
blocked until the maintainer reviews this report and formally accepts Stage 03.

## 2. Objectives and acceptance criteria

| Objective or criterion | State | Evidence |
| --- | --- | --- |
| Add exactly one workspace member, `ghostram-planner`; no other crate or Stage 04 work. | Met | Workspace lists exactly five crates; no engine, UI, store, or tuner crate was added. |
| Keep planning calculations pure: metadata, resource snapshot, and validated options in; no scanning, probing, subprocesses, filesystem traversal, network, or engine calls. | Met | `ghostram-planner` has no I/O or process dependency. The CLI performs file inspection and light memory/NVIDIA observation before calling the pure core. |
| Add `ghostram models plan <path>` with human and `--json` output, predictable errors/exit codes, and no change to `doctor` or `models scan` behavior. | Met | CLI unit/integration tests cover help, argument errors, human/JSON output, malformed files, missing paths, and unchanged existing commands. |
| Require an explicitly selected GGUF file; never auto-select, rescan directories, hash automatically, persist plans, or use a cache/database. | Met | `inspect_file` is called with one path. There is no directory scan, hash call, store, or cache in the plan path. Missing, directory, and symlink paths fail with exit 1. |
| Use existing parser/probe contracts; preserve partial structured plans when optional probes fail. | Met | Plan uses `FileCatalogEntry`, `ghostram_probe::collect_planning_observations`, and emits structured evidence/unknowns when host or NVIDIA observation is missing. |
| Distinguish typed evidence: observed, explicit input, derived, estimated, product default, unknown; never turn missing metadata into zero or call an estimate a measurement. | Met | `EvidenceClass` has all six classes. Unknown variants have no `value`; tests cover missing host memory, malformed files, and default labeling. |
| Treat GGUF file size only as a low-confidence proxy for resident weights; do not infer exact tensor allocation, quantization overhead, or GPU layer counts. | Met | `WeightMethod::GgufFileSizeProxy`; caveats and tests state the limitation. Shard and malformed files leave total weights unknown. |
| Use checked integer arithmetic and explicit byte units; no floating-point canonical storage or unchecked conversions. | Met | `Bytes(u64)` and checked add/sub/mul paths; overflow tests return unknown, not wrapped values. JSON byte values are integers. |
| Support documented uniform K/V architecture profiles only; unsupported/special layouts or missing dimensions produce explicit unknown/unavailable KV results. | Met | Only `llama_uniform_full_attention_v1` is accepted. Recognized sliding-window/SWA markers, unsupported architectures, arrays, missing/zero dimensions, inconsistent heads, and overflow produce typed `unavailable`. |
| Support F16 and F32 KV assumptions, visibly default to F16; no naive quantized KV calculations. | Met | `KvCacheType::{F16,F32}` with 2/4 bytes; default is `product_default` F16. Quantized values are rejected by CLI validation. |
| Implement configurable host/device reserves and non-negative effective budgets; document conservative product-policy defaults. | Met | `--ram-reserve-mib` and `--vram-reserve-mib`; defaults 1,536 MiB and 1,024 MiB. Tests cover equal, above, unknown, and overflow cases with no negative budget. |
| Evaluate only defensible scenarios: CPU-resident weights+KV and hypothetical full-device weight placement; use qualified outcomes. | Met | Exactly `cpu_weights_and_kv` and `device_weights_cpu_kv`; outcomes are limited to within, exceed, and insufficient data. Partial offload and GPU KV are not modeled. |
| Never emit unqualified `model_fits`, `launchable`, `supported`, or `safe_to_run`; state launch success and backend workspace remain unverified. | Met | Purity tests check the JSON; caveats state the limits. The plan outcome vocabulary contains only qualified scenario outcomes. |
| Define versioned `ghostram.models.plan/1` without changing `ghostram.doctor/1` or `ghostram.models.scan/1`; no secrets or metadata dumps. | Met | Envelope rejects foreign schemas; doctor and scan schemas/tests are unchanged. Plan model summary omits tokenizer values, chat templates, prompts, tensor bytes, and arbitrary metadata. |
| Add deterministic unit and CLI integration tests covering arithmetic, reserves, boundaries, overflow, formulas, profiles, missing/unknown metadata, cache types, proxy labeling, device uncertainty, JSON round-trips, malformed files, output purity, and exit codes. | Met | 212 workspace tests passed, including 27 planner unit and 19 planner integration tests and 10 new CLI plan tests. |
| Add a dated research note, focused planner specification, complete 14-section Stage 03 report, and required documentation updates. | Met | Research note, `MEMORY_PLANNER.md`, this report, schema, README, changelog, status, roadmap, architecture, testing, catalog, engine-support, contributor, and user docs were updated. |
| Record Stage 02 acceptance dated 2026-10-21 and Stage 03 authorization; remove stale Stage 02-pending statements. | Met | Stage 02 report and current docs record acceptance and authorization. Historical authoring-time text is explicitly retained where applicable. |
| Update Stage 04 planning for CPU baseline, official Linux CUDA 12.8 Pascal candidate, official Vulkan candidate, and source build targeting `sm_61`, without installing or running an engine. | Met | Roadmap and engine-support planning list the four candidates. No installation, launch, inference, benchmark, driver, or toolchain change was made. |
| Do not download/install models or engines; do not run inference/benchmarks; do not change drivers/toolchains/system settings; do not tag, release, or promote `main`. | Met | No such action was performed. Publication is limited to `dev` after validation. |

## 3. Summary of completed and incomplete work

Completed:

- Added the fifth crate, `ghostram-planner`, with pure modules for evidence,
  checked budgets, KV estimation, resource normalization, options, components,
  scenarios, warnings, unknowns, and the versioned envelope.
- Added single-file inspection in `ghostram-models` and planning-only observations
  in `ghostram-probe`. The full `doctor` probe set is not run for a plan.
- Added parsed architecture-field and attention-layout presence markers. They are
  internal planning inputs and are skipped by serialization, so the Stage 02 JSON
  contract is unchanged.
- Added `ghostram models plan` parsing, dispatch, human rendering, JSON output,
  help, errors, and exit codes without altering `doctor` or `models scan` behavior.
- Added deterministic synthetic GGUF and resource fixtures for planner unit,
  planner integration, and CLI integration tests.
- Added the research note, planner behavior specification, plan schema, Stage 03
  report, and documentation/status updates.
- Recorded Stage 02 acceptance and Stage 03 authorization, and updated Stage 04
  planning candidates.

Incomplete or deferred, with reasons:

- **No real GGUF model or engine validation.** No real model was available, and
  installing/downloading one was not authorized. The file-size proxy and KV
  formula remain uncalibrated against an actual engine allocation.
- **No runtime/backend overhead estimate.** Backend workspace, graph allocations,
  compute buffers, driver overhead, alignment, and mmap behavior are not modeled;
  the plan labels runtime overhead unknown.
- **No quantized KV cache.** Quantized formats require block/layout/scale and
  backend-specific treatment, not a naive divisor.
- **No candidate lists, tuner, or Pareto search.** These are later-stage features.
- **No launch refusal, override, or runtime pressure governor.** Stage 03 has no
  launch path, so it reports `insufficient_data` or a qualified estimate instead of
  pretending to enforce a runtime policy.
- **No plan persistence, history, cache, database, or stable model ID.**
- **No shard grouping, multi-file model, adapter, or sidecar combination.**
- **No non-NVIDIA device memory planning, partial offload, GPU layer prediction,
  multi-GPU planning, or GPU-resident KV scenario.**
- **No release, tag, GitHub Release, or `main` promotion.**

## 4. Architectural decisions and rationale

No ADR was added, changed, or superseded. The fifth crate follows the crate-split
rule in [ADR-0002](../../docs/adr/0002-modular-monolith-and-crate-split.md), and no
accepted decision was rewritten.

| Decision | Rationale |
| --- | --- |
| Make `ghostram-planner` a crate rather than a module. | The pure calculation contract has a distinct test seam, a versioned JSON boundary, and future CLI/UI adapters. It does not need filesystem, probe, or engine dependencies. |
| Keep inspection/observation in adapters. | Purity makes arithmetic deterministic and testable. The CLI composes `inspect_file`, lightweight probes, and `build_plan` explicitly. |
| Use `Evidence<T>` with six classes. | Every quantity exposes how it was obtained. `product_default` is separate from user input and observation; unknown has no value. |
| Use only a file-size weight proxy. | No tensor allocation or quantization model was available. The proxy is honest about its low confidence and refuses shard totals. |
| Support one KV profile. | Upstream layouts are too varied for a safe generic formula without a loader. Special markers and unsupported architectures become typed unavailable. |
| Track marker presence without serializing it. | Stage 03 needs to detect special layouts, while Stage 02's `ghostram.models.scan/1` contract must remain unchanged. |
| Use F16 default and explicit F32. | F16 is upstream server default and the conservative visible policy; F32 is a simple exact alternative. Quantized cache modeling is deferred. |
| Use non-negative budgets and qualified outcomes. | Missing data cannot become zero, and a modeled comparison cannot become a launch verdict. |
| Keep plan JSON at `ghostram.models.plan/1`. | It is a separate contract from doctor and scan, with additive-compatible fields and explicit unknown objects. |
| Defer launch refusal and runtime pressure control. | There is no engine process to govern in Stage 03. Adding a fake refusal rule would not make a launch safe. |

## 5. Files added, changed, or removed

Added:

- `crates/ghostram-planner/Cargo.toml`
- `crates/ghostram-planner/src/lib.rs`
- `crates/ghostram-planner/src/error.rs`
- `crates/ghostram-planner/src/evidence.rs`
- `crates/ghostram-planner/src/budget.rs`
- `crates/ghostram-planner/src/kv.rs`
- `crates/ghostram-planner/src/resources.rs`
- `crates/ghostram-planner/src/plan.rs`
- `crates/ghostram-planner/tests/planner.rs`
- `crates/ghostram-probe/src/planning.rs`
- `docs/MEMORY_PLANNER.md`
- `docs/PLAN_SCHEMA.md`
- `docs/research/2026-10-21-kv-cache-planning.md`
- `reports/stages/stage-03-memory-planner.md` (this report)

Changed:

- `Cargo.toml`, `Cargo.lock`: add the planner workspace member and internal
  dependency.
- `crates/ghostram-cli/Cargo.toml`: add the planner dependency.
- `crates/ghostram-cli/src/main.rs`: parse, validate, dispatch, and document
  `models plan`; collect planning observations; write JSON/human output.
- `crates/ghostram-cli/src/render.rs`: render plan sections and evidence.
- `crates/ghostram-cli/tests/cli.rs`: planner fixture and integration tests.
- `crates/ghostram-models/src/catalog.rs`: architecture/layout marker types and
  non-serialized presence fields; custom equality for the serialized contract.
- `crates/ghostram-models/src/gguf.rs`: classify and retain marker presence.
- `crates/ghostram-models/src/lib.rs`: export the marker types and inspection API.
- `crates/ghostram-models/src/scan.rs`: `inspect_file` and tests.
- `crates/ghostram-probe/src/lib.rs`: export planning observations.
- Documentation and policy files listed in section 13.
- `reports/stages/stage-01-minimal-rust-workspace.md` and
  `reports/stages/stage-02-gguf-model-catalog.md`: record acceptance status without
  changing the validation evidence.
- `scripts/check_docs.py`: require the planner docs, research note, and Stage 03
  report.

Removed: none. No user file, model, engine, benchmark, or unrelated work was
deleted or overwritten.

## 6. Dependencies added or modified

- Added the internal workspace dependency `ghostram-planner` to the CLI.
- The planner depends only on `ghostram-core`, `ghostram-models`, and `serde`.
  `serde_json` is a dev-dependency for tests. No external runtime crate was added.
- `Cargo.lock` changed only to include the new workspace member and its existing
  workspace dependency edges; no external package version was added by the planner.
- No Rust toolchain, CUDA toolkit, Vulkan runtime, driver, engine, or system
  dependency was installed or modified.
- `ghostram-models` gained no new dependency for `inspect_file`; it reuses the
  existing bounded parser and scan policy.
- `ghostram-probe` gained no new dependency for `collect_planning_observations`; it
  reuses the existing memory and NVIDIA collectors.

## 7. Validation commands and observed results

Run from the repository root. Selected observed output is below; binary paths and
timings vary between runs.

```text
$ cargo fmt --all
(no output)

$ cargo fmt --all -- --check
(no output)

$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking ghostram-planner v0.1.0-dev (/myfiles/r31nu11/Work/GitHub/GhostRAM/crates/ghostram-planner)
    Checking ghostram-cli v0.1.0-dev (/myfiles/r31nu11/Work/GitHub/GhostRAM/crates/ghostram-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.63s

$ cargo test --workspace --locked
     Running unittests src/main.rs (target/debug/deps/ghostram-73b52cd834ee7166)
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/cli.rs (target/debug/deps/cli-971b28f0f496bc86)
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
     Running unittests src/lib.rs (target/debug/deps/ghostram_core-d1eb17db9a9b8e76)
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/ghostram_models-9ebf5a3c3d501acb)
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
     Running unittests src/lib.rs (target/debug/deps/ghostram_planner-0ae6cf815d036891)
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/planner.rs (target/debug/deps/planner-772c60f176e92aa3)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/ghostram_probe-20affc2a8f9a9e69)
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
   Doc-tests ghostram_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ghostram_models
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ghostram_planner
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests ghostram_probe
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ cargo build --workspace --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s

$ python3 scripts/check_docs.py
ok: 37 Markdown files, 208 relative links, 32 required files

$ bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD
# Output is abridged to the Stage 01/02/03 tail; all earlier commits in the range
# were also accepted.
check: ed96c19ea3a8 feat(stage-01): add diagnostic workspace and doctor command
ok: commit message accepted
check: 02ad8bb8a836 docs(stage-01): record validation and content commit
ok: commit message accepted
check: 887fba442b48 fix(probe): bound timeout cleanup to isolated process groups
ok: commit message accepted
check: 244809bf12fd feat(stage-02): add bounded GGUF model catalog
ok: commit message accepted
check: 4143ac648c71 docs(stage-02): record validation and content commit
ok: commit message accepted
check: 2dda210a28f8 feat(planner): add deterministic memory planning
ok: commit message accepted
ok: all checked commit messages passed

$ git diff --check
(no output)

$ git config --local --get user.name
r31nu11

$ git config --local --get user.email
r31nu11@mail.ru
```

The full test run had no failures. `cargo test` also built the workspace test
targets; the locked build command observed above is a separate check. Clippy
warnings are denied. No test was skipped or reported as passed without execution.
The command above was observed with the Stage 03 content commit as `HEAD`. The
final hand-off reruns the suite and commit-range check over the completed history;
the final self-referential HEAD result is reported outside this commit.

## 8. Hardware tests and measurements

No hardware test or measurement was performed for Stage 03.

- No real GGUF model was available or downloaded.
- No engine was installed or launched.
- No CUDA or Vulkan computation, inference, allocation observation, or benchmark
  was run.
- No GPU layer count, peak VRAM, resident RAM, KV allocation, backend workspace, or
  runtime overhead was measured.
- The CLI integration tests run the lightweight memory/NVIDIA observation path with
  a restricted `PATH` to exercise missing-probe behavior. Those runs are test
  plumbing, not a hardware validation record and are not reported as measured
  performance.
- The primary laptop remains the Stage 01 observation: 4 GiB Pascal GPU and
  7.4 GiB RAM. The secondary GTX 1070 machine has not been accessed.

All planner evidence is therefore synthetic: deterministic GGUF metadata, synthetic
resource snapshots, and unit/integration assertions. No estimate in this stage is a
measurement.

## 9. Tests that could not be performed

| Test | Missing prerequisite | Risk of leaving it untested |
| --- | --- | --- |
| Parse and plan a real GGUF model | A user-supplied or explicitly approved model; none was available. | The parser and formula may miss real family defaults, metadata variants, or tensor layouts. |
| Compare file-size proxy with actual resident weights | An engine, a real model, and a memory sampler. | Weight estimates may be materially wrong even when the arithmetic is correct. |
| Compare KV formula with an engine allocation | A pinned engine build, a real model, and backend allocation observation. | Backend padding, alignment, and cache layout may change the true cache size. |
| Launch/smoke test CPU, CUDA 12.8, Vulkan, or `sm_61` build | Engine/toolchain installation and explicit approval. | No launchability or backend-compatibility claim can be made. |
| Measure runtime/backend workspace and overhead | A running engine and process/device sampling. | `within_budget` remains only about modeled weights and KV, not actual headroom. |
| Quantized KV, sliding-window, latent, recurrent, or non-uniform layouts | Additional architecture profiles and upstream evidence. | These inputs produce explicit unavailability; users cannot plan them yet. |
| Multi-device, AMD/Intel, and Vulkan device-memory planning | Device probes and a memory model beyond NVIDIA. | Only NVIDIA device budgets are available. |
| Candidate generation and Pareto search | Later-stage planner/tuner design and measurements. | Stage 03 cannot answer "which configuration is best"; it only compares modeled components. |
| Launch refusal and runtime pressure shutdown | An engine adapter and supervised runtime. | No runtime safety enforcement exists yet. |
| Plan persistence/history | A later storage design and schema. | Plans must be regenerated; no cross-run comparison exists. |

## 10. Known bugs, limitations, and unresolved questions

Known limitations:

- File size is a low-confidence weight proxy, not resident memory. It excludes
  tensor offsets/alignment, quantization block overhead, mmap/page-cache behavior,
  sidecars, and GPU placement.
- The KV formula is narrow: uniform full-attention `llama`, F16/F32, independent
  sequences, full requested context on every layer. Padding, workspace, mixed
  layers, sliding windows, latent/recurrent state, and quantized caches are not
  modeled.
- `device_weights_cpu_kv` is hypothetical full-device weight placement with host
  KV. It does not model partial offload, GPU-resident KV, or engine placement.
- The plan has no runtime/backend overhead value. `within_budget` is not launch
  approval.
- Device selection is NVIDIA-only and defaults to the first observed index. There
  is no Vulkan/AMD/Intel memory budget, and device presence does not prove backend
  compatibility.
- A missing host `MemAvailable` or NVIDIA field remains unknown; a partial plan can
  be CPU-informative while the device scenario is `insufficient_data`.
- The context upper bound uses the host effective budget after the weight proxy; it
  is not a model, engine, or backend context limit and can exceed declared context
  with a warning.
- Plans are ephemeral. There is no cache, database, history, stable model ID, shard
  grouping, or sidecar combination.
- Stage 03 has no launch path, so the earlier roadmap idea of refusing unsafe
  candidates is not implemented. This is recorded as deferred, not as a hidden
  safety guarantee.
- `ArchitectureFields` custom equality compares only serialized fields; marker
  presence is intentionally not restored by deserialization. This avoids changing
  the Stage 02 JSON contract and is covered by a round-trip test.
- The parser stores marker presence by recognized dotted suffix, then filters it to
  the selected architecture. An unrecognized special-layout key outside the
  documented marker set is not detected; the narrow profile remains a preflight
  estimate, not a full validator.

Unresolved questions:

1. How close is the file-size proxy to actual resident weights on a real model?
2. Does the uniform formula match the engine's per-layer cache allocation after
   padding/alignment/workspace?
3. Which additional architecture profiles and quantized cache types can be added
   without guessing?
4. Should future planning use unified sequence cache sharing instead of independent
   full contexts?
5. Which device observations are reliable enough for Vulkan and other backends?
6. What runtime reserve/refusal policy should Stage 04 implement, and how should an
   override be recorded?

## 11. Current branch and commit hash

- Branch: `dev`.
- Content commit: `2dda210a28f88bee4dc6c2538db9224ca1cdb5d6`.
- Final report/status commit: the next `docs(stage-03)` commit. Its hash is
  self-referential here; use `git log -1` after hand-off.
- Both commits use the maintainer identity `r31nu11 <r31nu11@mail.ru>`.
- The commits are local until the final hand-off pushes `dev` to `origin/dev`.
- No tag, GitHub Release, artifact, or `main` promotion is created.

## 12. Git working-tree status

At content-commit staging, `git status --short` contained only the Stage 03 code,
test, documentation, policy, schema, research, and report additions/modifications.
The content commit contains those paths. Build output under `target/` is ignored.

After the final report/status commit, the working tree is expected to be clean.
The final hand-off verifies `git status --short` and reports the observed result
outside the self-referential commit.

## 13. Documentation updated

- `README.md`: Stage 02 acceptance, Stage 03 planner status, capabilities, source
  commands, schemas, and safety boundaries.
- `AGENTS.md`: accepted Stage 01/02, Stage 03 state, five-crate map, planner
  non-claims, report links, and Stage 03 validation commands.
- `CHANGELOG.md`: `Unreleased` planner, schema, tests, acceptance, and security
  entries.
- `docs/PROJECT_STATUS.md`: Stage 03 snapshot, capabilities, decisions, validation,
  environment, deferrals, and next-stage gate.
- `docs/ROADMAP.md`: Stage 02 accepted; Stage 03 delivered and pending acceptance;
  Stage 04 candidate planning; superseded scope reconciliation.
- `docs/ARCHITECTURE.md`: planner crate, pure boundary, five-crate dependency
  direction, evidence contract, plan contract, data flow, and future runtime
  governor separation.
- `docs/TESTING.md`: Stage 03 checks, planner unit/integration/contract tests,
  synthetic evidence, and untested hardware/allocation cases.
- `docs/GGUF_CATALOG.md`: Stage 02 acceptance, single-file inspection, planner
  handoff, marker serialization boundary, and planner limitations.
- `docs/MODELS_SCHEMA.md`: Stage 02 acceptance and no Stage 03 change to
  `ghostram.models.scan/1`.
- `docs/ENGINE_SUPPORT.md`: Stage 02/03 status, Stage 04 CPU/CUDA 12.8/Vulkan/`sm_61`
  candidate plan, and the planner's non-claims.
- `docs/CONTRIBUTING.md`: Stage 03 workflow status and planner implementation.
- `docs/INSTALL.md`, `docs/QUICK_START.md`, `docs/TROUBLESHOOTING.md`,
  `docs/UNINSTALL.md`: source-only planner commands, options, errors, and absence of
  installed state.
- `docs/RELEASING.md`: Stage 03 publication status without a tag or release.
- `docs/MEMORY_PLANNER.md` (new): pure boundary, CLI/options, evidence classes,
  weight proxy, KV profile/formula, budgets, scenarios, context bound, determinism,
  and limitations.
- `docs/PLAN_SCHEMA.md` (new): complete `ghostram.models.plan/1` contract, evidence
  objects, fields, enum codes, compatibility rules, and representative example.
- `docs/research/2026-10-21-kv-cache-planning.md` (new): pinned upstream sources,
  formula derivation, profile boundaries, confidence labels, open questions, and
  non-claims.
- `reports/stages/stage-01-minimal-rust-workspace.md`: acceptance status recorded.
- `reports/stages/stage-02-gguf-model-catalog.md`: acceptance dated 2026-10-21 and
  Stage 03 authorization recorded without changing evidence.
- `scripts/check_docs.py`: required planner docs, research note, and Stage 03
  report.

The documentation check result is recorded in section 7 after the final run. No
documentation path was knowingly left stale; the only retained pending language is
the current Stage 03 acceptance state and explicitly labeled historical report
text.

## 14. Recommended next stage

Maintainer review and formal acceptance of Stage 03 are recommended next. Stage 04
(engine adapter and supervised runtime) is the dependent next stage but must not
start until Stage 03 is accepted and separately authorized. Its planning entry is
[the roadmap Stage 04 section](../../docs/ROADMAP.md#stage-04--engine-adapter-and-supervised-runtime).

Before any engine installation or launch, Stage 04 must obtain separate maintainer
approval for a small model, an engine artifact, and the exact test environment. Its
candidate evaluation should cover the CPU baseline, official Linux CUDA 12.8 Pascal
candidate, official Vulkan candidate, and a source build targeting `sm_61` with a
CUDA 12.x toolkit. None of those candidates is installed, pinned, or verified by
Stage 03. Tags, releases, artifact publication, and `main` promotion remain separate
decisions.

---

## Completion checklist

- [x] Acceptance criteria checked with evidence.
- [x] Validation commands and observed results recorded.
- [x] `PROJECT_STATUS.md` and `ROADMAP.md` updated.
- [x] Changelog updated for user-visible behavior.
- [x] No ADR added or superseded; rationale recorded in section 4.
- [x] Working tree state explained; final hand-off verifies cleanliness.
- [ ] Maintainer acceptance recorded (pending formal review).
- [x] No push, tag, release, install, or system change without approval.
