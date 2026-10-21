# Testing strategy

**Status:** Stage 03 (pending formal acceptance). The Rust workspace, read-only
diagnostic probes, bounded GGUF metadata parser, model scanner, pure memory planner,
CLI, parser and planner fixtures, unit tests, integration tests, and Rust CI job
exist. There is still no engine adapter, model loader, inference, or benchmark
harness to test. No real GGUF model was available; model and planner tests use
deterministic synthetic files and synthetic resource snapshots.

## 1. Principles

1. A check is not a test. The scripts in `scripts/` validate repository
   conventions; they do not validate inference behavior.
2. Tests must be reproducible without a GPU whenever possible. Hardware-specific
   behavior needs an explicit manual test record.
3. Model files and engine binaries are untrusted inputs. Parsers and probes must
   handle truncation, malformed metadata, missing tools, and unexpected output
   without panicking.
4. Unit tests may use fixtures. Integration tests must not require a real model
   unless the test is explicitly marked as hardware-dependent.
5. A skipped test is reported as skipped. It is never counted as passed.
6. Benchmarks are experiments, not tests. Their methodology and evidence rules
   are in [BENCHMARKING.md](BENCHMARKING.md).

## 2. Current checks (Stage 03)

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
bash scripts/check-commit-message.sh <path-to-message-file>
git diff --check
```

| Check | What it proves | What it does not prove |
| --- | --- | --- |
| `cargo fmt --check` | Rust source matches the workspace formatting. | Code quality or behavior. |
| `cargo clippy ... -D warnings` | No warnings from the configured Clippy lint set. | Absence of logic bugs, security issues, or hardware compatibility. |
| `cargo test --workspace` | Diagnostic parser fixtures, GGUF parser/scan fixtures, planner arithmetic/formula/evidence tests, report models, bounded-command behavior, CLI argument handling, and CLI integration tests pass. | That real engines, models, CUDA/Vulkan inference, allocations, or runtime overhead work as estimated. |
| `cargo build --workspace --locked` | The workspace builds from the committed dependency lockfile. | That a release artifact exists or that another platform builds. |
| `check_docs.py` | Required files exist; relative Markdown links resolve; ADRs have required fields; stage reports have all 14 sections. | That prose is accurate, that external URLs are live, or that instructions work. |
| `check-commit-message.sh` | One message follows the header/body convention and has `Changes:`, `Reason:`, and `Validation:` bullets. | That the change is correct or that validation claims are true. |
| `check-commit-range.sh` | Commits in a range follow the convention. | That the range is the right comparison base. |
| `git diff --check` | No whitespace errors are introduced in tracked changes. | Markdown correctness or code quality. |

The observed results of these commands for Stage 03 are recorded in the
[Stage 03 report](../reports/stages/stage-03-memory-planner.md). Stage 01 and
Stage 02 results remain in their
[Stage 01 report](../reports/stages/stage-01-minimal-rust-workspace.md) and
[Stage 02 report](../reports/stages/stage-02-gguf-model-catalog.md). Do not repeat
a result here without re-running the command.

## 3. Test layers

### 3.1 Unit tests

Implemented in Stages 01, 02, and 03:

- `Bytes` conversion, overflow, display, and integer JSON serialization.
- Diagnostic-state and envelope behavior, including schema rejection.
- Parsers for captured `/etc/os-release`, `/proc/cpuinfo`, `/proc/meminfo`,
  `/proc/swaps`, `df`, `nvidia-smi`, `vulkaninfo --summary`, and llama.cpp
  version/device output.
- Malformed and incomplete fixture cases that must return errors or missing fields
  rather than guessed values.
- Bounded-command success, non-zero exit, output truncation, process-group timeout
  cleanup, direct-child reaping, inherited-pipe descendant cleanup, and bounded
  return when a descendant deliberately escapes the process group.
- Synthetic GGUF v2/v3 fixtures covering all documented scalar metadata types,
  nested arrays, boolean validation, tokenizer array summaries, tensor types, and
  architecture ordering.
- GGUF rejection tests for bad magic, every truncated header prefix, unsupported
  versions/endianness/types, duplicate metadata keys and tensor names, invalid
  alignment and dimensions, offsets outside the data region, oversized values, and
  representative entry, key, string, array, tensor, dimension, and distinct-type
  limit cases.
- Scanner tests for non-recursive/recursive traversal, case-insensitive suffixes,
  sorted output, symlink roots and entries, non-UTF-8 candidates, unreadable
  candidates, empty directories, deterministic filename/shard hints, summary
  consistency, change detection, and optional hash success/failure/no-partial
  behavior.
- Planner unit tests for checked byte arithmetic, evidence classes, file-size proxy
  labeling, KV formula and dimension provenance, F16/F32 scaling, unsupported
  layouts, missing or unusable metadata, zero/edge values, overflow, reserves,
  non-negative budgets, comparison precedence, scenario ordering, warning and
  unknown de-duplication, and envelope round-trips.
- Contract serialization proving token values, merge values, and chat-template
  text are absent, plus CLI argument parsing and human rendering.

Planned for later stages:

- Real-model parser and planner cross-check with an approved model and engine.
- Real allocation calibration for weights, KV cache, backend workspace, and
  runtime overhead.
- Runtime configuration resolution: intent-to-flag mapping and provenance.

Unit tests use small fixtures, either checked-in parser captures or deterministic
byte builders, not live hardware. A parser test must include at least one malformed
or truncated fixture.

### 3.2 Integration tests

Implemented in Stages 01, 02, and 03:

- CLI valid JSON on stdout with no human-readable noise.
- Valid partial JSON when `PATH` makes optional tools unavailable.
- Exit code 2 for invalid arguments, and help/version behavior.
- Report generation completes when optional GPUs, Vulkan devices, and engines are
  absent.
- `models scan` help, empty-directory success, non-recursive versus recursive
  behavior, human/JSON output, root failure exit 1, and no hash unless requested.
- Model-scan JSON stays well formed for mixed valid/malformed directories, and a
  hash result is absent unless the complete file was read.
- `models plan` help, missing path/unknown option/duplicate option usage errors,
  explicit file selection, missing/directory/symlink failures, malformed-file
  success with partial plan, human/JSON output, output purity, context/sequence/
  cache/reserve/device options, and exit codes.
- Planner integration scenarios for CPU and device budgets, missing host or device
  observations, device selection, unsupported KV layout, shard/sidecar hints,
  shortfalls, reserve exhaustion, and JSON round-trips.

Planned for later stages:

- Process lifecycle with a fake engine executable: clean start, health timeout,
  crash, signal handling, graceful stop, and forced kill of a process group.
- HTTP client behavior against a local fake server: health, properties, streaming
  completion, malformed responses, and connection loss.
- Atomic report or run-record writes.

Integration tests must be hermetic: no network access beyond loopback, no
downloads, no package installation, and no real GPU requirement. The Stage 01 CLI
tests do not require a GPU, engine, or model.

### 3.3 Contract tests

- The adapter contract is tested with a fake adapter and snapshot tests for the
  argument array.
- Engine-specific parsers are tested against captured real output from pinned
  builds. These fixtures are labeled with engine build and capture date.
- Capability claims distinguish `supported`, `unsupported`, and `unknown`; a
  missing probe must not become `supported`.
- Planner JSON preserves evidence tags, rejects unqualified fit/launch terms, and
  keeps unknown quantities distinct from zero. This is a schema/purity test, not
  an allocation or engine-safety proof.

### 3.4 Hardware tests

Hardware tests are manual and recorded, not hidden in CI:

- `ghostram doctor` on the primary laptop: performed in Stage 01 and recorded in
  the Stage 01 report.
- The same diagnostic run on the secondary GTX 1070 machine: not performed; the
  machine was not accessed.
- Engine discovery when no engine is installed: observed as absent on the primary
  laptop.
- Device enumeration with both Intel and NVIDIA Vulkan adapters visible: observed
  on the primary laptop.
- Real-model GGUF parsing or hashing: not performed; no real model was available
  or downloaded. Stage 02 uses synthetic files in temporary directories.
- Real-model memory planning or allocation comparison: not performed. Stage 03
  uses synthetic GGUF metadata and synthetic resource snapshots; it did not measure
  weights, KV cache, backend workspace, or runtime overhead.
- A small-model smoke run on CPU and at least one GPU backend, only with
  maintainer approval: not performed; no engine or model is installed.
- Cleanup after success, failure, timeout, and interrupt for a real engine: not
  performed. Stage 01 tests cleanup only for bounded read-only subprocesses.

A hardware test record includes the date, machine, OS/kernel, driver, engine
build, model hash, exact command, observed output, and whether the result was
reproduced.

### 3.5 Benchmark validation

Benchmark runs need their own validation: manifest completeness, unit checks,
sample counts, variance, and consistency between raw and summarized values. See
[BENCHMARKING.md](BENCHMARKING.md) and
[reports/benchmarks/README.md](../reports/benchmarks/README.md).

## 4. Local and CI sequence

The minimum local and CI sequence is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
python3 scripts/check_docs.py
```

CI also retains the documentation and commit-message jobs. The Rust job does not
run GPU tests or download engines or models.

## 5. Test data and secrets

- Do not commit model weights, engine binaries, API keys, tokens, or user paths
  that identify a person.
- Fixtures derived from real command output must be reduced to the fields under
  test and stripped of hostnames, usernames, and secrets.
- Do not store large raw logs in Git. Store bounded excerpts and a hash of the
  original where needed.
- Tests must not write outside a temporary directory and must clean up child
  processes.

## 6. CI

The current workflow (`.github/workflows/quality.yml`) runs documentation checks,
commit-range checks, and a Rust job with formatting, Clippy, tests, and a locked
build. At Stage 03 authoring time the workflow had been parsed locally but had not
executed on GitHub Actions. The development history was later published to
`origin/dev`, but no hosted-runner result has been observed or recorded by this
hand-off; do not infer CI success from the push. CI results do not replace local
validation recorded in the stage report. GPU tests will not run in hosted CI; they
require a self-hosted machine and an explicit opt-in label.

CI must not download models, install CUDA, or run benchmarks. A green CI result
is not evidence about inference performance.

## 7. Failure and flakiness policy

- A flaky test is a defect. Marking it ignored requires a linked issue and an
  assigned maintainer.
- Timeouts must be explicit and generous enough for slow CI, but a timeout is a
  failure unless the test is specifically testing timeout behavior.
- Tests that depend on wall-clock performance use loose bounds and are never the
  sole evidence for a product claim.
- If a test cannot run in an environment, record the reason and the missing
  prerequisite instead of silently skipping it.

## 8. Reporting

Every stage report records the exact commands run, their observed outcome, and
the tests that could not be performed. A statement such as "tests pass" without
the command and result is not acceptable evidence.
