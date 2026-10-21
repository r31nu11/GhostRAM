# Changelog

All notable changes to GhostRAM will be documented in this file. The project
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

No GhostRAM version has been released or tagged. `Unreleased` now includes the
accepted Stage 01 diagnostic implementation, the accepted Stage 02 source model
catalog, and the Stage 03 source memory planner pending formal acceptance. It
remains a development snapshot, not a release artifact or supported installation.

## [Unreleased]

### Added

- Rust workspace with exactly five crates: `ghostram-core`, `ghostram-models`,
  `ghostram-planner`, `ghostram-probe`, and `ghostram-cli`, plus a committed
  `Cargo.lock`.
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
- `ghostram-models`, a read-only bounded GGUF v2/v3 little-endian parser that
  extracts compact general, architecture, tokenizer-summary, tensor-type, and
  alignment metadata without parsing tensor payloads.
- `ghostram models scan <path>` with non-recursive default, `--recursive`,
  `--json`, safe symlink handling, deterministic ordering, partial-scan
  diagnostics, and versioned `ghostram.models.scan/1` output.
- Explicit optional streaming SHA-256 with a fixed 64 KiB buffer, change
  detection, no partial digest reporting, and no implicit full-file reads.
- `ghostram-planner`, a pure calculation crate with evidence-tagged quantities,
  checked byte arithmetic, non-negative effective budgets, configurable host and
  device reserves, and two qualified scenarios. The workspace now contains exactly
  five crates.
- `ghostram models plan <path>` for one explicitly selected GGUF file, with human
  and `ghostram.models.plan/1` JSON output. It does not scan directories, hash
  automatically, persist plans, or call an engine.
- A narrow `llama_uniform_full_attention_v1` KV-cache profile for uniform
  full-attention `llama` files, with visible F16 default, explicit F32 support,
  checked overflow handling, and typed unavailability for missing dimensions or
  special attention layouts. Quantized KV caches are not modeled.
- A low-confidence GGUF file-size weight proxy, an optional budget-limited context
  upper bound, explicit runtime-overhead unknown, and outcomes limited to
  `estimated_components_within_budget`, `estimated_components_exceed_budget`, and
  `insufficient_data`.
- Deterministic planner unit and CLI integration tests covering arithmetic,
  reserves, boundaries, overflow, formula/profile behavior, missing metadata,
  cache types, proxy labeling, device uncertainty, JSON round-trips, malformed
  files, output purity, and exit codes.
- Deterministic GGUF fixtures and adversarial tests covering metadata types,
  nested arrays, duplicate keys and tensor names, truncation, invalid alignment,
  resource limits, tokenizer summaries, unknown tensor types, symlinks, hash
  behavior, change detection, and CLI output.
- Deterministic parser fixtures and tests for OS release, CPU info, meminfo, swap,
  df, nvidia-smi, Vulkan summary, llama.cpp version/device output, malformed input,
  partial reports, and CLI behavior.
- Rust formatting, Clippy, test, and locked-build jobs in CI.
- [Diagnostic report schema](docs/DOCTOR_SCHEMA.md), [GGUF catalog behavior](docs/GGUF_CATALOG.md),
  [model scan schema](docs/MODELS_SCHEMA.md), and dated GGUF format research.
- [Memory planner behavior](docs/MEMORY_PLANNER.md), [plan schema](docs/PLAN_SCHEMA.md),
  and dated KV-cache planning research.
- Stage 01, Stage 02, and Stage 03 stage reports; Stage 01 and Stage 02 acceptance
  are recorded.
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
- Accepted Stage 01 on 2026-10-21 and authorized Stage 02 model-catalog work.
- Accepted Stage 02 on 2026-10-21 and authorized Stage 03 memory-planner work.
- Updated architecture, testing, installation, quick-start, troubleshooting,
  engine-support, status, roadmap, and contributor documentation for the Stage 01
  diagnostic implementation.
- Added the planner boundary, versioned plan contract, KV-cache research, and
  Stage 03 test coverage without changing `ghostram.doctor/1` or
  `ghostram.models.scan/1`.
- Reconciled the superseded Stage 02 wording: the delivered scope has no metadata
  cache, `models list`, shard grouping, or real-model/engine cross-check.
- Documented that the catalog parser follows the GGUF specification's
  multiple-of-8 alignment and UTF-8 key handling rather than adopting every
  additional restriction in the current upstream loader.

### Fixed

- Stage 01 subprocess timeouts now terminate the isolated process group on Unix
  and drain output pipes with a bounded grace. A descendant that inherits
  stdout/stderr can no longer keep the caller blocked after the direct child is
  killed.
- Stage 01 parser handling avoids zero, guessed, or "supported" values for missing
  or inconclusive measurements. Missing optional tools and hardware remain
  explicit `partial` or `unavailable` results.
- Stage 02 tokenizer extraction now treats `tokenizer.ggml.added_tokens` as
  `array[string]`, matching upstream GGUF rather than an earlier integer-array
  assumption.
- Stage 02 alignment validation now rejects every present non-`uint32`
  `general.alignment` value, including float, string, boolean, and array forms,
  instead of falling back to the default.
- Stage 02 chat-template and Hugging Face JSON presence flags now remain accurate
  when those keys are present with an unexpected scalar type.

### Removed

- Nothing.

### Security

- Added an initial security disclosure policy; no release support matrix exists
  yet.
- Model scanning is read-only, skips symlinks, uses non-following candidate opens
  on Unix, enforces checked resource limits, and never emits tokenizer values,
  chat templates, prompts, or tensor bytes. The schema retains only compact
  metadata and optional hashes.
- Plan output contains compact metadata and absolute selected paths, but no
  tokenizer values, chat templates, prompts, tensor bytes, or arbitrary metadata
  blobs. Plans are ephemeral and no model file is written.
