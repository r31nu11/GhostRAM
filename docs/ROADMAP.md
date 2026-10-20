# Roadmap

The roadmap is ordered by dependency, not by calendar date. Each stage ends with
a report, updated status, and human review. A stage may be split or reordered by
the human owner; it may not be declared complete without its acceptance criteria.

## Status legend

- **Complete (pending review):** work and report exist; owner acceptance pending.
- **Next:** the recommended next stage.
- **Planned:** defined but not started.
- **Later:** intentionally deferred.

## Stage overview

| Stage | Title | State | Depends on |
| --- | --- | --- | --- |
| 00 | Technical reconnaissance and project charter | Complete (pending review) | — |
| 01 | Minimal Rust workspace and diagnostic spine | Next | 00 |
| 02 | GGUF model catalog and identity | Planned | 01 |
| 03 | Memory planner and safety budgets | Planned | 02 |
| 04 | Engine adapter and supervised runtime | Planned | 01, 03 |
| 05 | Benchmark harness and reports | Planned | 03, 04 |
| 06 | Tuner, history, and Pareto search | Planned | 05 |
| 07 | Iced desktop MVP | Planned | 05 |
| 08 | Packaging, installation, and alpha release candidate | Planned | 06, 07 |

## Stage 00 — Technical reconnaissance and project charter

**State:** Complete pending human review.

**Delivered:** starting-state and environment observations; engine/backend
research with sources; initial integration strategy; architecture boundaries;
MVP and non-goals; benchmark methodology; branch/commit/release workflow;
documentation contracts; Stage 00 report.

**Gate:** owner reviews the report, accepts or changes the proposed ADRs, and
authorizes the next stage.

## Stage 01 — Minimal Rust workspace and diagnostic spine

**Goal:** prove the toolchain and produce a trustworthy read-only environment
report without touching engines or models.

**Scope:**

- Create a Cargo workspace with only the crates justified by ADR-0002; expected:
  `ghostram-core`, `ghostram-probe`, `ghostram-cli`.
- Define typed units, IDs, errors, and a versioned JSON envelope in core.
- Implement Linux probes for OS/kernel, CPU, RAM, swap/zram, NVIDIA GPU and
  driver, Vulkan devices, storage, and relevant tools.
- Implement engine *discovery* (paths and `--version`/`--list-devices` probes)
  without launching inference.
- Add `ghostram doctor` with human-readable and JSON output.
- Add unit tests with fixtures/mocks for parsers; manual run on the laptop.
- Add Rust formatting, linting, and test checks to CI.
- Update status, roadmap, and Stage 01 report.

**Acceptance:**

- `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test` pass.
- `ghostram doctor` runs on the primary laptop and reports the observed GPU,
  driver, RAM, swap, and Vulkan devices.
- Missing engines/models are reported as absent, not as errors.
- No packages installed, no models downloaded, no system settings changed.
- Report states what was not verified.

**Non-goals:** GGUF parsing, memory estimates, engine launch, benchmarks, GUI.

## Stage 02 — GGUF model catalog and identity

**Goal:** discover models safely and understand their metadata before launching
anything.

**Scope:**

- Add a model crate/module with a bounded, read-only GGUF metadata parser.
- Scan configured directories; handle split GGUF shards; identify sidecars.
- Record path, size, modification time, SHA-256, architecture, parameter count,
  context length, layer/head counts, quantization, tokenizer metadata, and
  shard information.
- Cache metadata with invalidation by size/mtime/hash policy.
- Add `ghostram models list` and JSON output.
- Test malformed and truncated files; do not parse tensor payloads unnecessarily.
- Update status, roadmap, and Stage 02 report.

**Acceptance:** parser survives malformed fixtures without panics; metadata for
at least one real model is cross-checked against engine-reported metadata in a
later stage; no model files are modified.

## Stage 03 — Memory planner and safety budgets

**Goal:** generate candidate configurations with explicit budgets and uncertainty.

**Scope:**

- Implement weight, KV-cache, and backend-overhead estimates from metadata.
- Separate predictions, resolved configurations, and measurements.
- Add RAM/VRAM budget policy and refusal/override rules.
- Produce candidate lists for goals: speed, balanced, long context, largest model.
- Add tests for GQA/MQA, quantized KV, zero/edge contexts, overflow, and
  architecture-specific cases.
- Calibrate against measurements only when Stage 05 data exists; until then label
  estimates as uncalibrated.
- Update status, roadmap, and Stage 03 report.

**Acceptance:** estimates are monotonic where expected, assumptions are visible,
unsafe candidates are refused by default, and no estimate is presented as a
measurement.

## Stage 04 — Engine adapter and supervised runtime

**Goal:** launch, observe, and stop a real engine safely.

**Scope:**

- Define the engine adapter contract and build-capability model.
- Implement the official llama.cpp adapter via a child process and HTTP API.
- Launch `llama-server` on loopback with an API key, explicit model/flags, and
  captured logs; wait for health; enforce startup and idle timeouts.
- Stop the process group on request, timeout, crash, or resource pressure.
- Record resolved arguments, build info, devices, and exit status.
- Add a fake engine for deterministic tests.
- Run a smoke test on the laptop only with owner authorization and a small model.
- Update status, roadmap, and Stage 04 report.

**Acceptance:** no orphan processes after success, failure, or timeout; engine
capabilities are observed per build; logs and resolved configuration are stored;
no inference optimizer is implemented yet.

## Stage 05 — Benchmark harness and reports

**Goal:** produce reproducible throughput and resource measurements.

**Scope:**

- Wrap `llama-bench` JSON for controlled prompt/generation throughput.
- Measure client-side TTFT through streaming `llama-server` requests.
- Sample VRAM/RAM/swap/PSI; record peaks and sampling intervals.
- Record model/engine/backend/device/config/workload identity.
- Emit versioned JSON and generated Markdown; keep raw samples.
- Run a small CPU and GPU matrix (with owner approval and small models).
- Compare CUDA 12.8, Vulkan, and CPU if builds are available.
- Update status, roadmap, and Stage 05 report.

**Acceptance:** at least one complete run record is reproducible on the same
machine; variance and failures are reported; no fabricated or cherry-picked
results; differences between prompt processing and generation are explicit.

## Stage 06 — Tuner, history, and Pareto search

**Goal:** use bounded experiments and history to recommend configurations.

**Scope:**

- Add a store for run records and hardware/model fingerprints.
- Implement a bounded candidate search (grid or surrogate) with safety limits.
- Warm-start from compatible historical measurements; record transfer assumptions.
- Rank by declared goals and present a Pareto frontier.
- Add a usability threshold that rejects swap-heavy or extremely slow runs.
- Compare search against fixed heuristics in a pre-declared experiment.
- Update status, roadmap, and Stage 06 report.

**Acceptance:** search never exceeds resource budgets; all recommendations link
to measured evidence; history improves or fails to improve the search with an
honest result.

## Stage 07 — Iced desktop MVP

**Goal:** expose stable core services through a native desktop UI.

**Scope:**

- Add `ghostram-ui` with Iced 0.14.x (or the then-current compatible release).
- Views: environment, models, plan/candidates, active run, results/history.
- Keep long-running work off the UI thread; show cancellation and errors.
- Reuse the same services and JSON records as the CLI; no duplicate logic.
- Verify Wayland/X11 behavior and integrated/discrete GPU selection.
- Update status, roadmap, and Stage 07 report.

**Acceptance:** UI can perform the MVP flow without the CLI; no engine work is
implemented in the UI; accessibility and failure states are documented.

## Stage 08 — Packaging, installation, and alpha release candidate

**Goal:** let an ordinary user install and run GhostRAM without Rust or a compiler.

**Scope:**

- Choose and document the Linux packaging format(s).
- Build self-contained artifacts, checksums, licenses, and third-party notices.
- Finalize install/quick-start/update/uninstall instructions.
- Validate on a clean environment and the primary laptop.
- Define engine/runtime acquisition and version pinning.
- Produce a release candidate, changelog, compatibility statement, and known
  limitations; do not publish without owner approval.
- Update status, roadmap, and Stage 08 report.

**Acceptance:** a clean-machine install and uninstall works; no development
toolchain is required; artifacts and checksums are verified; release notes are
accurate; owner approves promotion.

## Later / unscheduled

- Windows support and CUDA/Vulkan packaging.
- Secondary GTX 1070 validation.
- Additional engine adapters (only with demonstrated user value).
- PrismML-specific packings and drafters as an optional adapter.
- Model download management with explicit consent and checksums.
- Advanced search, quality guards, and cross-machine transfer.
- Performance improvements that require upstream engine changes (out of scope
  unless explicitly authorized).

## Roadmap rules

- Do not start a stage before its dependencies are accepted.
- Do not implement GUI work before the CLI/services it needs are stable.
- Do not add crates or abstractions without a boundary and a test seam.
- Update this file at the end of every stage with actual state, not intentions.
