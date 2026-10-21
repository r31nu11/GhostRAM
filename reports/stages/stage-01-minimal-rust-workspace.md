# Stage 01 — Minimal Rust workspace and diagnostic spine

**Date:** 2026-10-20
**Maintainer:** `r31nu11` (project maintainer)
**Branch:** `dev`
**Status:** Accepted on 2026-10-21 (implementation complete and evidence retained)

## 1. Stage identifier and title

Stage 01 — Minimal Rust workspace and diagnostic spine. This report covers the
implementation, tests, documentation, and local validation for that stage. The
maintainer accepted Stage 01 on 2026-10-21; the original pre-acceptance wording is
retained only where it describes the authoring-time state.

## 2. Objectives and acceptance criteria

The objective was to prove the Rust toolchain and produce a trustworthy,
read-only environment report without installing or launching an engine or model.

| # | Criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Create only the crates justified by ADR-0002: `ghostram-core`, `ghostram-probe`, `ghostram-cli`. | Met | Workspace has exactly those three members; see `Cargo.toml` and section 5. |
| 2 | Define typed byte quantities, system/hardware models, diagnostic states, typed errors, and a versioned serializable report envelope in core. | Met | `ghostram-core` modules and 13 passing unit tests. |
| 3 | Implement read-only probes for OS/kernel, CPU, RAM, swap/zram, storage, NVIDIA GPU/driver, Vulkan adapters, tools, and external llama.cpp discovery. | Met | `ghostram-probe` modules, fixtures, and 44 passing unit tests. The command ran successfully on the primary laptop. |
| 4 | Bound and audit subprocesses; use explicit paths and argument arrays; no shell interpolation; clean up timeouts; never convert inconclusive results into support. | Met for Linux | `command.rs` uses explicit `Command` argument arrays, places each child in its own process group, bounds stdout/stderr memory, polls pipes with nonblocking reads, and on timeout sends `SIGKILL` to the group before reaping the direct child. Deterministic tests cover direct timeout/reap, inherited-pipe descendant cleanup, bounded return when a descendant deliberately escapes the group, output truncation, success, and non-zero exits. Unavailable/partial categories retain reasons. |
| 5 | Provide `ghostram doctor`, `doctor --json`, help/version, meaningful exit codes, and useful output when optional tools are absent. | Met | CLI unit and integration tests pass; missing-tool and partial-report tests pass; invalid arguments return 2. |
| 5a | Missing engines and models are reported as absent, not as errors. | Met for engines; model discovery is out of scope | The engines category reports an available empty result with a reason. Model discovery and absence reporting are deferred to Stage 02, so no model claim is made. |
| 6 | JSON includes schema version, application version, generation time, system data, category states, unavailable reasons, and discovered tool/engine paths/versions. | Met | `ghostram.doctor/1` envelope; real JSON was parsed and inspected. |
| 7 | Add deterministic parser/unit tests, malformed-input tests, partial-report tests, and CLI integration tests. | Met | 69 tests pass in total: 13 core, 44 probe, 5 CLI unit, and 7 CLI integration. |
| 8 | Add Rust CI checks while retaining documentation and commit-message jobs. | Met | `.github/workflows/quality.yml` retains both existing jobs and adds `rust` with format, Clippy, test, and locked build steps. YAML parsed locally. |
| 9 | Update required project documentation and create a Stage 01 report with all 14 sections. | Met | Documentation changes are listed in section 13; this report contains all template sections. |
| 10 | Run the required checks and record exact results. | Met | Section 7 records the commands and observed output. |
| 11 | Do not install an engine or model, download anything, launch inference, benchmark, or change system settings. | Met | No install, download, inference, benchmark, driver, package, or settings command was run. Tool discovery only read `PATH` and executed bounded version/device queries. |
| 12 | Preserve unrelated work and avoid push, tag, release, or `main` changes. | Met | Work remained on `dev`; the remote refs were not changed. See sections 11 and 12. |

Formal maintainer acceptance is deliberately not marked met. That is the review
gate, not an implementation task.

## 3. Summary of completed and incomplete work

Completed:

- Created the three-crate Rust workspace and committed dependency lockfile.
- Implemented exact byte quantities, diagnostic categories and states, system
  models, typed errors, the `ghostram.doctor/1` envelope, and schema validation.
- Implemented read-only Linux collection for OS, CPU, memory, swap, zram,
  storage, NVIDIA GPU, Vulkan, tools, and llama.cpp executable discovery.
- Implemented bounded command execution with output limits and timeouts, isolated
  process groups, group termination on timeout, and bounded output draining.
- Implemented human and JSON output, argument validation, help, version, and
  exit codes.
- Added parser fixtures and tests for valid, incomplete, malformed, and partial
  input, plus CLI integration tests.
- Added the Rust CI job and updated documentation checks to validate all stage
  reports.
- Updated project documentation for Stage 00 acceptance, Stage 01 reality,
  diagnostic schema, validation, limitations, and next-stage gating.

Incomplete or deferred:

- Acceptance was still pending at authoring time; the maintainer accepted Stage 01
  on 2026-10-21.
- No release artifact, installer, or package exists.
- No real llama.cpp engine, engine build hash, or model was present, so engine
  execution and inference are unverified.
- GGUF parsing, the model catalog, memory planner, engine adapter, benchmark
  harness, tuner, UI, and packaging remain later-stage work.
- The secondary GTX 1070 machine was not accessed.
- GitHub Actions has not executed the new job because no push was authorized.

## 4. Architectural decisions and rationale

No new ADR was required for the diagnostic implementation. Stage 00's
ADR-0001 through ADR-0005 were accepted on 2026-10-20 and continued to govern the
work:

- [ADR-0002](../../docs/adr/0002-modular-monolith-and-crate-split.md) justified the
  three-crate split and the rule that the engine adapter arrives later.
- [ADR-0003](../../docs/adr/0003-engine-integration-strategy.md) kept integration
  process-based and discovery read-only.
- [ADR-0005](../../docs/adr/0005-pinned-engine-builds.md) separates build identity
  from installation records and runtime capability observations. Stage 01 only
  records discovery data; it does not create an engine build identity.

The new diagnostic contract is `ghostram.doctor/1`, implemented as typed models
in `ghostram-core` and documented in
[DOCTOR_SCHEMA.md](../../docs/DOCTOR_SCHEMA.md).
A breaking schema change requires a new identifier. There is no engine or
measurement contract yet, so no additional ADR was invented.

## 5. Files added, changed, or removed

Added:

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | Workspace definition and locked dependency graph. |
| `crates/ghostram-core/Cargo.toml` | Core crate manifest. |
| `crates/ghostram-core/src/lib.rs` | Public module surface. |
| `crates/ghostram-core/src/quantity.rs` | Exact `Bytes` quantity and conversions. |
| `crates/ghostram-core/src/error.rs` | Typed errors and stable error codes. |
| `crates/ghostram-core/src/diagnostic.rs` | Diagnostic states, system models, envelope, and schema validation. |
| `crates/ghostram-probe/Cargo.toml` | Probe crate manifest. |
| `crates/ghostram-probe/src/lib.rs` | Public report entry point. |
| `crates/ghostram-probe/src/report.rs` | Report assembly and unsupported-platform behavior. |
| `crates/ghostram-probe/src/command.rs` | Bounded subprocess execution. |
| `crates/ghostram-probe/src/os.rs` | `/etc/os-release` and kernel collection. |
| `crates/ghostram-probe/src/cpu.rs` | `/proc/cpuinfo` collection. |
| `crates/ghostram-probe/src/memory.rs` | `/proc/meminfo` and `/proc/swaps` collection. |
| `crates/ghostram-probe/src/zram.rs` | zram sysfs collection. |
| `crates/ghostram-probe/src/storage.rs` | Bounded `df` collection for `/`. |
| `crates/ghostram-probe/src/nvidia.rs` | Bounded `nvidia-smi` collection. |
| `crates/ghostram-probe/src/vulkan.rs` | Bounded `vulkaninfo --summary` collection. |
| `crates/ghostram-probe/src/tools.rs` | `PATH` lookup and tool version queries. |
| `crates/ghostram-probe/src/engine.rs` | llama.cpp executable discovery. |
| `crates/ghostram-probe/src/time.rs` | UTC RFC 3339 generation timestamp. |
| `crates/ghostram-probe/tests/fixtures/*` | Deterministic OS, CPU, memory, swap, df, NVIDIA, Vulkan, and llama.cpp fixtures. |
| `crates/ghostram-cli/Cargo.toml` | CLI crate manifest. |
| `crates/ghostram-cli/src/main.rs` | Argument parsing, output, and exit codes. |
| `crates/ghostram-cli/src/render.rs` | Human-readable rendering. |
| `crates/ghostram-cli/tests/cli.rs` | CLI integration tests. |
| `docs/DOCTOR_SCHEMA.md` | Versioned diagnostic JSON, privacy, bounds, and compatibility contract. |
| `reports/stages/stage-01-minimal-rust-workspace.md` | This report. |

Changed:

| Path | Change |
| --- | --- |
| `.gitignore` | Corrected the build-output comment now that the workspace exists. |
| `.github/workflows/quality.yml` | Added the Rust format, Clippy, test, and locked-build job. |
| `scripts/check_docs.py` | Requires the diagnostic schema, Cargo files, and Stage 01 report; validates every stage report. |
| `AGENTS.md` | Records Stage 00 acceptance, Stage 01 implementation, the workspace map, and Rust validation commands. |
| `CHANGELOG.md` | Records the Stage 01 implementation under `Unreleased`, not a release. |
| `README.md` | Adds source-development commands, current diagnostic state, and release caveats. |
| `docs/PROJECT_STATUS.md` | Replaces the Stage 00 snapshot with Stage 01 implementation state and validation. |
| `docs/ROADMAP.md` | At authoring time, marked Stage 00 accepted and Stage 01 complete pending acceptance; Stage 02 was then next but blocked. Stage 01 was accepted on 2026-10-21. |
| `docs/ARCHITECTURE.md` | Describes the implemented diagnostic path and corrects engine identity and unit scope. |
| `docs/ENGINE_SUPPORT.md` | Adds Stage 01 discovery behavior and clarifies that discovery is not support. |
| `docs/TESTING.md` | Records current Rust checks, test layers, hardware tests, and CI. |
| `docs/CONTRIBUTING.md` | Applies the workflow to code and lists the Rust checks. |
| `docs/INSTALL.md`, `docs/QUICK_START.md`, `docs/TROUBLESHOOTING.md` | Distinguish source diagnostics from a nonexistent installation. |
| `docs/PROJECT_CHARTER.md` | Records acceptance of the stable mission. |
| `docs/RELEASING.md` | Records Stage 00 acceptance and the absence of a release artifact. |
| `docs/adr/0001` … `docs/adr/0005`, `docs/adr/README.md` | Mark the decisions Accepted and describe their status. |
| `reports/stages/stage-00-technical-reconnaissance.md` | Records acceptance and authorization, with historical Stage 00 facts labeled as such. |

Removed: none.

## 6. Dependencies added or modified

Direct dependencies:

| Dependency | Version | Kind | Reason |
| --- | --- | --- | --- |
| `serde` with `derive` | 1.0.229 | Runtime, `ghostram-core` | Typed JSON serialization of the diagnostic contract. |
| `serde_json` | 1.0.154 | Runtime, `ghostram-cli`; dev, `ghostram-core` | JSON generation and test round-trips. |
| `libc` | 0.2.191 | Runtime, `ghostram-probe` on Unix | Minimal binding for `kill(2)`; `std::process` can create a process group but does not expose group signalling. No C build step was added. |
| `ghostram-core` | path | Runtime, probe and CLI | Shared contracts and models. |
| `ghostram-probe` | path | Runtime, CLI | Read-only collection. |

Transitive locked packages are `serde_core`, `serde_derive`, `proc-macro2`,
`quote`, `syn`, `unicode-ident`, `itoa`, `memchr`, and `zmij`. `libc` adds only
the platform C-library bindings already used by the Rust runtime; no GPU toolkit,
engine, model, native-build dependency, or new system package was added.

Toolchain observed: `rustc 1.94.1`, `cargo 1.94.1`, stable x86_64. The workspace
declares Rust 2024 edition and `rust-version = "1.85"`. `Cargo.lock` is committed
and all build/test commands used `--locked`.

## 7. Validation commands and observed results

All commands were run from the repository root. Output below is summarized from
the observed terminal output; exit status was 0 unless stated.

### 7.1 Formatting

```text
$ cargo fmt --all -- --check
(no output; exit 0)
```

### 7.2 Clippy

```text
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking ghostram-cli v0.1.0-dev
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
```

### 7.3 Tests

```text
$ cargo test --workspace --locked
5 passed (CLI unit tests)
7 passed (CLI integration tests)
13 passed (ghostram-core unit tests)
44 passed (ghostram-probe unit tests)
0 passed (doc-tests)
Total: 69 passed; 0 failed; 0 ignored
```

The focused lifecycle run below was repeated five times; all eight command-module
tests passed on every run.

```text
$ cargo test -p ghostram-probe --lib command::tests -- --nocapture
running 8 tests
test command::tests::subprocess_lifecycle_helper ... ok
test command::tests::bounds_both_output_streams ... ok
test command::tests::reports_nonzero_exit_without_a_timeout ... ok
test command::tests::captures_output_and_success ... ok
test command::tests::truncates_captured_output ... ok
test command::tests::terminates_and_reaps_a_timed_out_child ... ok
test command::tests::terminates_an_inherited_pipe_descendant_on_timeout ... ok
test command::tests::bounded_return_survives_a_descendant_that_escapes_the_process_group ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out; finished in 2.51s
```

The output test writes more than the 64 KiB stdout and 16 KiB stderr limits and
verifies that both captured strings stop exactly at those limits. The timed-out
tests use a Rust test-helper process rather than a shell. They assert an outer
return bound of 8 seconds, verify the direct child is reaped by checking that
`/proc/<pid>` disappears, and verify a descendant in the same process group is
also gone. The escaped-descendant test deliberately creates a new process group,
observes a bounded return, verifies that the escaped process survived, and then
kills it and waits for it to disappear as test cleanup. That test records the
containment limitation rather than claiming absolute process-tree control.

### 7.4 Locked build

```text
$ cargo build --workspace --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
```

### 7.5 Real diagnostic run

```text
$ cargo run -q -p ghostram-cli -- doctor
GhostRAM doctor
Application: 0.1.0-dev
Schema: ghostram.doctor/1
Generated: 2026-10-20T19:43:09Z
Overall: available
...
NVIDIA GPUs:
  [0] NVIDIA GeForce GTX 1050 Ti: VRAM 4.0 GiB total, 3.0 MiB used;
      driver 580.178.04; compute 6.1
Vulkan:
  instance: 1.4.363
  [0] Intel(R) UHD Graphics 630 (CFL GT2) ...
  [1] NVIDIA GeForce GTX 1050 Ti ...
Engines:
  no llama.cpp executables found on PATH
```

### 7.6 JSON output

```text
$ cargo run -q -p ghostram-cli -- doctor --json > /tmp/opencode/doctor-stage01.json
$ python3 -m json.tool /tmp/opencode/doctor-stage01.json > /dev/null
(exit 0)

schema_version = ghostram.doctor/1
application_version = 0.1.0-dev
generated_at = 2026-10-20T19:43:09Z
overall_state = available
stdout_bytes = 4464
non-JSON stdout lines = 0
engines = []
```

All ten categories were `available`; the engines category carried the explanation
`no llama.cpp executables were found on PATH`.

### 7.7 Documentation, repository, and workflow checks

```text
$ python3 scripts/check_docs.py
ok: 29 Markdown files, 122 relative links, 24 required files

$ bash -n scripts/check-commit-message.sh scripts/check-commit-range.sh .githooks/commit-msg
(no output; exit 0)

$ python3 (PyYAML BaseLoader parse of .github/workflows/quality.yml)
YAML parsed. top-level keys: ['jobs', 'name', 'on', 'permissions']
jobs: ['commit-messages', 'documentation', 'rust']
commit-messages steps: 2
documentation steps: 3
rust steps: 6

$ git diff --check
(no output; exit 0)

$ git diff --cached --check
(no output after staging; exit 0)

$ bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD
check: ... (all non-merge local commits in the range, including ed96c19ea3a8)
ok: all checked commit messages passed

$ git config --local --get user.name
r31nu11
$ git config --local --get user.email
r31nu11@mail.ru
```

The commit-message checker fixtures were rerun during final validation; the valid
message was accepted and the invalid message was rejected with exit 1. The result
is recorded in the final hand-off because the fixtures are not part of the Rust
test count. The range check was run again after the final safety-follow-up commit
was created; the hand-off records the final observed result. No fixed count of
local commits is recorded here because the count changes with each follow-up.

## 8. Hardware tests and measurements

This stage performed a read-only diagnostic run, not an inference or performance
test.

| Item | Observation |
| --- | --- |
| Machine | Primary development laptop. |
| OS / kernel | CachyOS (rolling), kernel `7.2.11-1-cachyos`, x86_64. |
| CPU | Intel Core i7-8750H, 6 physical cores / 12 logical CPUs. |
| RAM | 7,989,465,088 bytes total; 3,470,852,096 bytes available at the report run. |
| Swap | `/dev/zram0`, 7,989,096,448 bytes, zstd, no disk swap device. |
| Storage | `/dev/sdb2` mounted at `/`: 124,895,825,920 bytes total, 43,790,991,360 bytes available. |
| NVIDIA GPU | GeForce GTX 1050 Ti, 4,294,967,296 bytes VRAM, driver 580.178.04, compute 6.1. |
| Vulkan adapter 0 | Intel UHD Graphics 630, Mesa 26.2.4, Vulkan 1.4.354. |
| Vulkan adapter 1 | NVIDIA GTX 1050 Ti, driver 580.178.04, Vulkan 1.4.312. |
| Engine | None found on `PATH`. No engine build or model hash exists to record. |
| Command | `cargo run -q -p ghostram-cli -- doctor` and `... doctor --json`. |
| Reproduced | The diagnostic was generated more than once; volatile memory/storage values changed, while device, kernel, and driver identities remained stable. |

No GPU compute, CUDA, Vulkan inference, model load, benchmark, pressure, thermal,
or performance measurement was performed. The values above are observations, not
benchmark results.

## 9. Tests that could not be performed

| Test | Missing prerequisite | Risk of leaving it untested |
| --- | --- | --- |
| Real llama.cpp discovery and version/device parsing against an installed engine. | No engine installed; installation requires separate approval. | Parser fixtures may not cover a future executable's actual output or flag behavior. |
| CPU, CUDA, or Vulkan inference smoke test. | No engine or model installed. | No evidence that any backend executes inference on this laptop. |
| GGUF metadata and model-catalog tests. | Stage 02 not started. | Model fit and metadata handling remain unknown. |
| Memory-planner and budget tests. | Stage 03 not started. | Estimator correctness and safety behavior remain unknown. |
| Engine lifecycle, timeout, crash, signal, and orphan-process tests. | Stage 04 adapter not implemented. | The hardest runtime safety properties are not yet demonstrated. |
| Benchmark and pressure measurements. | No engine, model, or approved benchmark. | No performance or stability claims can be made. |
| Secondary GTX 1070 validation. | Machine was not accessed. | Cross-machine behavior is unknown. |
| GitHub Actions execution. | No push authorized. | The workflow YAML parsed locally, but hosted-runner behavior is unverified. |
| Non-Linux fallback report. | Running on Linux only. | The unsupported-platform path is implemented but not exercised end to end. |
| Deliberate process-group escape containment. | A descendant that calls `setsid()`, changes its process group, or enters another namespace is outside the group `SIGKILL`. | Such a descendant may survive a timeout. The bounded-drain test verifies that it cannot block return through inherited pipes; it does not provide containment. |
| Real partial/unavailable hardware failures. | Missing tools were tested through `PATH`; an installed-but-failing GPU tool was not induced. | Error paths for broken drivers or malformed real outputs are less exercised than fixtures. |
| Packaging, installation, uninstallation, and upgrade. | Stage 08 not started. | No release workflow has been validated. |

## 10. Known bugs, limitations, and unresolved questions

- The project has no release artifact, installer, or package. The CLI is
  source-buildable only.
- The implementation is Linux-specific. `collect_report` returns an unavailable
  report on other platforms, but that path was not exercised on a non-Linux host.
- `nvidia-smi` absence produces `nvidia_gpu: unavailable`; this does not prove that
  no NVIDIA GPU or driver exists.
- `vulkaninfo` absence or an unexpected summary format produces an unavailable or
  partial Vulkan category. The parser supports the observed summary structure, not
  every possible `vulkaninfo` version.
- Tool discovery checks a fixed list on `PATH`. A found executable can be reported
  without version text, and that does not prove the toolchain is usable.
- Engine discovery checks only `llama-server`, `llama-bench`, and `llama-cli` on
  `PATH`. It does not hash the executable, inspect linked backends, search arbitrary
  model/engine directories, or prove that inference works.
- The `engines` category is `available` with a reason when no engine is found. That
  is an intentional valid-empty result, not an error and not evidence of support.
- Storage collection covers `/` only. Model volumes and other mounts are not
  reported yet.
- On Unix, each probe runs in its own process group. A timeout sends `SIGKILL` to
  the group, then reaps the direct child. stdout and stderr are nonblocking and
  drained with a 500 ms grace after direct-child exit, so inherited pipes cannot
  cause an unbounded reader wait. A descendant that deliberately calls `setsid()`,
  changes its process group, or enters another namespace can escape group
  termination and survive. This is not absolute containment.
- On non-Unix platforms bounded subprocess execution returns an explicit error;
  `collect_report` reports the platform as unsupported before probing.
- Some outputs are locale- or version-dependent. The parsers use stable key and
  numeric fields where possible, but no exhaustive matrix of OS, driver, or Vulkan
  versions was tested.
- There is no persistent log, telemetry, crash bundle, or report-history store.
- The workspace declares `rust-version = "1.85"` to match Rust 2024, but only Rust
  1.94.1 was available locally. The declared MSRV and the locked dependency set
  were not tested on 1.85.
- No engine build or model has been verified, and no benchmark result exists. All
  performance and model-fit questions remain open.
- CUDA 13.x cannot build new Pascal device code; a CUDA 12.x toolchain is required
  for a Pascal-targeted CUDA build, with Vulkan as fallback. No such build was
  attempted in Stage 01.
- The 4 GiB VRAM / 7.4 GiB RAM budget is tight, and the desktop shares both
  resources. Usable model sizes and contexts are unknown until measured.
- Formal acceptance and the Stage 02 authorization are maintainer decisions and
  remain open.

## 11. Current branch and commit hash

**Branch:** `dev`.
**Base commit:** `7d3e41940a25bb3d9845478b603a359a098c22af` (`docs(adr-0005): stabilize engine build identity`).
**Content commit:** `ed96c19ea3a84f8c7e981a61f8c13764695eb13f`
(`feat(stage-01): add diagnostic workspace and doctor command`).
**Report/status commit:** `02ad8bb8a8360b0021f849f195be8bf26ef43365`
(`docs(stage-01): record validation and content commit`).
**Safety-follow-up commit:** the local commit that adds process-group timeout
cleanup, lifecycle tests, the `libc` dependency, and this correction. It is the
current HEAD when this report is read in a clean tree; the exact hash is visible
with `git log -1`.

All commits are local. No push, merge to `main`, tag, or GitHub Release was
performed. `origin/dev` and `origin/main` remained at
`bdfced9d7db49a1e2bdad5dd1602822734da33b1`.

## 12. Git working-tree status

At the start of Stage 01 the tree was clean at the base commit. During the work it
contained the new workspace and documentation changes. The content and
report/status commits followed. The safety follow-up started from a clean tree at
`02ad8bb8a8360b0021f849f195be8bf26ef43365`, changed the command runner, manifests,
lockfile, and the status/roadmap/schema/testing/report documentation, and was then
committed. The final hand-off records the observed clean state; any remaining path
would be unexpected and was not silently discarded.

## 13. Documentation updated

Updated:

- `README.md`, `CHANGELOG.md`, `AGENTS.md`.
- `docs/PROJECT_STATUS.md`, `docs/ROADMAP.md`, `docs/ARCHITECTURE.md`,
  `docs/DOCTOR_SCHEMA.md` (new), `docs/TESTING.md`, `docs/ENGINE_SUPPORT.md`,
  `docs/CONTRIBUTING.md`, `docs/INSTALL.md`, `docs/QUICK_START.md`,
  `docs/TROUBLESHOOTING.md`, `docs/PROJECT_CHARTER.md`, `docs/RELEASING.md`.
- `docs/adr/0001` through `docs/adr/0005` and `docs/adr/README.md` for acceptance
  status.
- `reports/stages/stage-00-technical-reconnaissance.md` for the accepted/authorized
  transition, while preserving its historical observations.
- `reports/stages/stage-01-minimal-rust-workspace.md` (this report).

The safety follow-up additionally corrected the subprocess-safety statements in
`PROJECT_STATUS.md`, `ROADMAP.md`, `DOCTOR_SCHEMA.md`, and `TESTING.md`, and
recorded the bounded timeout/cleanup behavior in `CHANGELOG.md`.

Not updated:

- `docs/BENCHMARKING.md`: no benchmark method was implemented or changed.
- `docs/UNINSTALL.md`: no installable artifact or data location was added.
- `docs/research/2026-10-20-engine-landscape.md`: no new external research evidence
  was gathered; the Stage 01 observations are implementation evidence, not a
  replacement for the dated research.

## 14. Recommended next stage

**Stage 02 — GGUF model catalog and identity.** It depends on Stage 01 acceptance
and explicit maintainer authorization. It should stay read-only toward model files
and must not download models or launch inference.

Before Stage 02 starts, the maintainer must:

1. Review and accept this report and the Stage 01 validation evidence.
2. Confirm that Stage 02's model-directory scanning and metadata cache scope match
   the project's privacy and read-only requirements.
3. Keep engine installation, model download, and the first smoke test deferred to
   Stage 04 or later with separate approval.

Roadmap: [Stage 02](../../docs/ROADMAP.md#stage-02--gguf-model-catalog-and-identity).

---

## Completion checklist

- [x] Acceptance criteria checked with evidence.
- [x] Validation commands and observed results recorded.
- [x] `PROJECT_STATUS.md` and `ROADMAP.md` updated.
- [x] Changelog updated when user-visible behavior changed.
- [x] ADRs added or superseded as needed; existing ADRs accepted.
- [x] Working tree clean after the report/status commit; confirmed in the final
  hand-off.
- [ ] Maintainer acceptance recorded.
- [x] No push, tag, release, install, or system change without approval.
