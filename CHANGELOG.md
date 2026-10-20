# Changelog

All notable changes to GhostRAM will be documented in this file. The project
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

No GhostRAM version has been released or tagged. `Unreleased` now includes the
Stage 01 source implementation and its read-only diagnostic CLI. It remains a
development snapshot, not a release artifact or supported installation.

## [Unreleased]

### Added

- Rust workspace with exactly three crates: `ghostram-core`, `ghostram-probe`, and
  `ghostram-cli`, plus a committed `Cargo.lock`.
- Typed byte quantities, diagnostic states, category reports, system models, typed
  errors, and a `ghostram.doctor/1` JSON envelope in `ghostram-core`.
- Read-only Linux probes for OS/kernel, CPU, memory, swap, zram, storage, NVIDIA
  GPU/driver, Vulkan devices, development/GPU tool availability, and external
  llama.cpp executable discovery.
- Bounded subprocess execution with explicit paths and argument arrays, output
  limits, timeouts, isolated process groups, and bounded group cleanup. No shell
  interpolation is used.
- `ghostram doctor` human-readable output and `ghostram doctor --json` with clean
  stdout, stable schema identifiers, and meaningful exit codes.
- Deterministic parser fixtures and tests for OS release, CPU info, meminfo, swap,
  df, nvidia-smi, Vulkan summary, llama.cpp version/device output, malformed input,
  partial reports, and CLI behavior.
- Rust formatting, Clippy, test, and locked-build jobs in CI.
- [Diagnostic report schema](docs/DOCTOR_SCHEMA.md) and the Stage 01 report.
- Project charter, MVP definition, non-goals, success criteria, and governance.
- Architecture proposal with crate boundaries, domain-model rules, and safety
  principles.
- Dated engine research covering official llama.cpp, CUDA/Pascal constraints,
  Vulkan, and PrismML's llama.cpp fork, with sources and confidence levels.
- Engine compatibility matrix and initial integration strategy.
- Benchmark methodology covering TTFT, prompt processing, generation, memory,
  swap pressure, load time, failures, controls, and reporting.
- `dev`/`main` branch policy, Conventional Commits template, local commit hook,
  commit-range validation, and documentation checks.
- Semantic Versioning, release-candidate, and promotion checklists.
- User installation, quick start, troubleshooting, and uninstall documentation
  contracts, explicitly marked as not yet available.
- Stage report template, Stage 00 report, and benchmark report layout.
- Architecture decision records for workflow, crate decomposition, engine
  integration, measurement provenance, and build identity.

### Changed

- Expanded `README.md` from a placeholder into the public project overview.
- Established `dev` as the development/integration branch; `main` remains the
  maintainer-accepted working/stable checkpoint branch.
- Recorded the Stage 00 project direction: no engine pinned or installed in
  Stage 01, no engine bundling in the MVP, and no universal speed threshold
  before measurements.
- Corrected the two unpublished Stage 00 commits to the maintainer's Git identity
  without changing their trees, messages, or order.
- Clarified that pushes, tags, and GitHub Releases require separate maintainer
  approval; stage acceptance is not a release.
- Standardized the documentation set on project voice and removed approval-history
  metadata from the ADRs.
- Refined ADR-0005 to separate immutable engine build identity from installation
  records and timestamped capability observations, and aligned `ARCHITECTURE.md`
  with that model.
- Accepted Stage 00 and adopted ADR-0001 through ADR-0005 on 2026-10-20.
- Updated architecture, testing, installation, quick-start, troubleshooting,
  engine-support, status, roadmap, and contributor documentation for the Stage 01
  diagnostic implementation.

### Fixed

- Stage 01 subprocess timeouts now terminate the isolated process group on Unix
  and drain output pipes with a bounded grace. A descendant that inherits
  stdout/stderr can no longer keep the caller blocked after the direct child is
  killed.
- Stage 01 parser handling avoids zero, guessed, or "supported" values for missing
  or inconclusive measurements. Missing optional tools and hardware remain
  explicit `partial` or `unavailable` results.

### Removed

- Nothing.

### Security

- Added an initial security disclosure policy; no release support matrix exists
  yet.
