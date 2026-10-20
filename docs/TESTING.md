# Testing strategy

**Status:** Stage 00 contract. Only the documentation and commit-message checks
exist today. There is no Rust workspace, binary, engine adapter, or benchmark
harness to test.

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

## 2. Current checks (Stage 00)

Run from the repository root:

```sh
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
bash scripts/check-commit-message.sh <path-to-message-file>
git diff --check
```

| Check | What it proves | What it does not prove |
| --- | --- | --- |
| `check_docs.py` | Required files exist; relative Markdown links resolve; ADRs have required fields; the Stage 00 report has all 14 sections. | That prose is accurate, that external URLs are live, or that instructions work. |
| `check-commit-message.sh` | One message follows the header/body convention and has `Changes:`, `Reason:`, and `Validation:` bullets. | That the change is correct or that validation claims are true. |
| `check-commit-range.sh` | Commits in a range follow the convention. | That the range is the right comparison base. |
| `git diff --check` | No whitespace errors are introduced in tracked changes. | Markdown correctness or code quality. |

The observed results of these commands for Stage 00 are recorded in the
[Stage 00 report](../reports/stages/stage-00-technical-reconnaissance.md). Do not
repeat a result here without re-running the command.

## 3. Planned test layers

### 3.1 Unit tests

- Pure domain types and unit conversions in `ghostram-core`.
- Parsers for captured command output: `/proc/meminfo`, `/proc/pressure/memory`,
  `nvidia-smi` query output, `vulkaninfo --summary`, `llama-server --version`,
  `llama-server --list-devices`, `llama-bench` JSON.
- Planner formulas: RAM/VRAM estimates, KV-cache scaling, overflow checks.
- GGUF metadata parser: bounds checking, invalid types, truncated headers.
- Configuration resolution: intent-to-flag mapping and provenance.

Unit tests use small checked-in fixtures, not live hardware. A parser test must
include at least one malformed or truncated fixture.

### 3.2 Integration tests

- Process lifecycle with a fake engine executable: clean start, health timeout,
  crash, signal handling, graceful stop, and forced kill of a process group.
- HTTP client behavior against a local fake server: health, properties, streaming
  completion, malformed responses, and connection loss.
- JSON envelope round-trips, schema versions, and atomic-write behavior.
- CLI exit codes and machine-readable output for known failure modes.

Integration tests must be hermetic: no network access beyond loopback, no
downloads, no package installation, and no real GPU requirement.

### 3.3 Contract tests

- The adapter contract is tested with a fake adapter and snapshot tests for the
  argument array.
- Engine-specific parsers are tested against captured real output from pinned
  builds. These fixtures are labeled with engine build and capture date.
- Capability claims distinguish `supported`, `unsupported`, and `unknown`; a
  missing probe must not become `supported`.

### 3.4 Hardware tests

Hardware tests are manual and recorded, not hidden in CI:

- `ghostram doctor` on the primary laptop and, later, the secondary GTX 1070
  machine.
- Engine discovery when no engine is installed.
- Device enumeration with both Intel and NVIDIA Vulkan adapters visible.
- A small-model smoke run on CPU and at least one GPU backend, only with
  maintainer approval.
- Cleanup after success, failure, timeout, and interrupt.

A hardware test record includes the date, machine, OS/kernel, driver, engine
build, model hash, exact command, observed output, and whether the result was
reproduced.

### 3.5 Benchmark validation

Benchmark runs need their own validation: manifest completeness, unit checks,
sample counts, variance, and consistency between raw and summarized values. See
[BENCHMARKING.md](BENCHMARKING.md) and
[reports/benchmarks/README.md](../reports/benchmarks/README.md).

## 4. Future commands

When the workspace exists, the minimum local and CI sequence will be:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_docs.py
```

These commands cannot run in Stage 00 because there is no `Cargo.toml` or Rust
source. Adding them to CI before the workspace exists would only produce a
failing placeholder job.

## 5. Test data and secrets

- Do not commit model weights, engine binaries, API keys, tokens, or user paths
  that identify a person.
- Fixtures derived from real command output must be reduced to the fields under
  test and stripped of hostnames, usernames, and secrets.
- Do not store large raw logs in Git. Store bounded excerpts and a hash of the
  original where needed.
- Tests must not write outside a temporary directory and must clean up child
  processes.

## 6. CI plan

The current workflow (`.github/workflows/quality.yml`) runs documentation checks
and commit-range checks. Stage 01 should add Rust format, Clippy, and test jobs.
GPU tests will not run in hosted CI; they require a self-hosted machine and an
explicit opt-in label.

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
