# Roadmap

The roadmap is ordered by dependency, not by calendar date. Each stage ends with
a report, updated status, and maintainer review. A stage may be split or reordered
by the maintainer; it may not be declared complete without its acceptance
criteria.

## Status legend

- **Accepted:** the maintainer has formally accepted the stage result.
- **Complete (pending acceptance):** work and report exist; maintainer acceptance
  pending.
- **Next:** the recommended next stage, subject to its approval gate.
- **Next (blocked):** the recommended next stage whose approval gate is not met.
- **Planned:** defined but not started.
- **Later:** intentionally deferred.

## Stage overview

| Stage | Title | State | Depends on |
| --- | --- | --- | --- |
| 00 | Technical reconnaissance and project charter | Accepted (2026-10-20) | — |
| 01 | Minimal Rust workspace and diagnostic spine | Accepted (2026-10-21) | 00 |
| 02 | GGUF model catalog and identity | Accepted (2026-10-21) | 01 |
| 03 | Memory planner and safety budgets | Complete (pending acceptance) | 02 |
| 04 | Engine adapter and supervised runtime | Next (blocked) | 01, 03 |
| 05 | Benchmark harness and reports | Planned | 03, 04 |
| 06 | Tuner, history, and Pareto search | Planned | 05 |
| 07 | Iced desktop MVP | Planned | 05 |
| 08 | Packaging, installation, and alpha release candidate | Planned | 06, 07 |

## Stage 00 — Technical reconnaissance and project charter

**State:** Accepted on 2026-10-20.

**Delivered:** starting-state and environment observations; engine/backend
research with sources; initial integration strategy; architecture boundaries;
MVP and non-goals; benchmark methodology; branch/commit/release workflow;
documentation contracts; Stage 00 report; ADR-0001 through ADR-0005.

**Gate:** met. The maintainer accepted the report and adopted the ADRs, then
authorized Stage 01.

## Stage 01 — Minimal Rust workspace and diagnostic spine

**State:** Accepted on 2026-10-21.

**Goal:** prove the toolchain and produce a trustworthy read-only environment
report without touching engines or models.

**Delivered:**

- Cargo workspace with exactly `ghostram-core`, `ghostram-probe`, and
  `ghostram-cli`, plus `Cargo.lock`.
- Typed byte quantities, diagnostic states, system models, typed errors, and a
  versioned `ghostram.doctor/1` envelope in core.
- Read-only Linux probes for OS/kernel, CPU, RAM, swap/zram, `/` storage, NVIDIA
  GPU and driver, Vulkan devices, relevant tools, and llama.cpp executable
  discovery.
- Bounded subprocess execution with explicit paths and argument arrays, output
  limits, timeouts, isolated process groups, and bounded group cleanup.
- `ghostram doctor` human-readable output and `ghostram doctor --json`.
- Deterministic parser fixtures, malformed-input tests, partial-report tests, and
  CLI integration tests.
- Rust formatting, Clippy, test, and locked-build checks in CI.
- Updated schema, status, roadmap, architecture, testing, user, and stage
  documentation.

**Acceptance evidence:** `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, and `cargo build --workspace --locked` passed.
The command was run on the primary laptop and reported the observed GPU, driver,
RAM, swap, and Vulkan devices. Missing engines were reported as an available empty
discovery result. No packages, engines, models, or settings were changed. Exact
results and unverified items are in the
[Stage 01 report](../reports/stages/stage-01-minimal-rust-workspace.md).

**Gate:** met. The maintainer accepted Stage 01 on 2026-10-21 and separately
authorized Stage 02. Acceptance did not itself authorize a push, tag, release, or
promotion to `main`.

**Non-goals:** engine installation, model download, inference launch, GGUF parsing,
memory estimates, benchmarks, GUI. Engine discovery may inspect `PATH` and run
read-only version/device probes, but must not install or launch inference.

## Stage 02 — GGUF model catalog and identity

**State:** Accepted on 2026-10-21.

**Goal:** let a user discover GGUF files in a directory they select and catalog
bounded, read-only metadata before any engine launch.

**Delivered:**

- A fourth workspace crate, `ghostram-models`, and the existing CLI extended with
  `ghostram models scan`.
- A bounded in-tree GGUF v2/v3 little-endian parser for headers, all documented
  metadata value types, nested arrays, tensor descriptors, alignment, and
  duplicate-key/name checks. Tensor payloads are not parsed.
- Compact extraction of general identity fields, common architecture counts,
  tokenizer array summaries and presence flags, tensor types, alignment
  provenance, and warnings. Token values, merges, chat templates, prompts, and
  arbitrary blobs are never emitted.
- Non-recursive directory scanning by default, optional `--recursive`, safe
  symlink handling, deterministic sorting, non-UTF-8 path diagnostics, bounded
  traversal, and per-file outcomes.
- Optional `--hash` streaming SHA-256 with a fixed buffer, complete-read
  requirement, and change/error detection that never reports a partial digest.
- Versioned `ghostram.models.scan/1` JSON output, human output, help, and exit-code
  contracts. `ghostram.doctor/1` is unchanged.
- Deterministic synthetic and adversarial tests for malformed, truncated,
  unsupported, resource-limit, tokenizer, tensor, scan, hash, symlink, and CLI
  cases.
- [GGUF catalog behavior](GGUF_CATALOG.md),
  [model scan schema](MODELS_SCHEMA.md), dated
  [GGUF format research](research/2026-10-21-gguf-catalog-format.md), and
  the [Stage 02 report](../reports/stages/stage-02-gguf-model-catalog.md).
- Updated README, changelog, policy, status, architecture, testing, engine-support,
  contributor, and user documentation.

**Acceptance evidence:** formatting, Clippy with warnings denied, all workspace
tests, the locked workspace build, documentation checks, commit-range checks, and
local identity checks passed. Exact commands, counts, and outcomes are recorded in
the Stage 02 report. The parser and scanner were tested only with synthetic
fixtures. No real model, engine, GPU, inference, or benchmark was used, and no
model files were written or modified.

**Gate:** met. The maintainer accepted Stage 02 on 2026-10-21 and separately
authorized Stage 03. Publication of the Stage 02 history to `origin/dev` did not
authorize a tag, GitHub Release, or promotion to `main`.

**Scope reconciliation:** the earlier Stage 02 wording named configured
directories, mandatory SHA-256, a metadata cache, `ghostram models list`, shard
grouping, and a real-model/engine cross-check. The maintainer-authorized scope
replaced configured directories with an explicit user-selected scan root, made
hashing optional, and deferred the cache, list command, grouping, and real-model
cross-check. Filename sidecar/shard hints are retained, but shards remain separate
records. These deferred items are not hidden incomplete requirements and must not
be claimed as delivered. A later authorized stage may add them with their own
tests and acceptance criteria.

**Non-goals:** model loading, inference, engine compatibility, shard merging,
persistent catalog state, model downloads, network services, write access, GUI, and
benchmarks.

## Stage 03 — Memory planner and safety budgets

**State:** Complete (pending formal acceptance). Stage 04 remains blocked until the
maintainer accepts this stage.

**Goal:** produce a deterministic, evidence-labelled, read-only memory plan for one
explicitly selected GGUF file and two defensible placement scenarios, without
loading the model or launching an engine.

**Delivered:**

- A fifth workspace crate, `ghostram-planner`, as a pure calculation core. It
  accepts a compact model summary, a resource snapshot, and validated options; it
  does not scan, probe, read files, start subprocesses, call an engine, or persist
  state.
- `ghostram models plan <path>` for one explicitly selected regular GGUF file,
  with human and `ghostram.models.plan/1` JSON output. The command does not scan a
  directory, hash automatically, use a default model path, or write a plan.
- Evidence classes `observed`, `explicit_input`, `derived`, `estimated`,
  `product_default`, and `unknown`. Unknown quantities carry a reason and no value
  field; missing data is never converted to zero, and an estimate is never called a
  measurement.
- A low-confidence file-size proxy for modeled weights, with shard, sidecar,
  malformed-file, and metadata-incomplete limitations visible. Exact tensor
  allocation, quantization overhead, mmap residency, and GPU layer counts are not
  inferred.
- One documented KV-cache profile, `llama_uniform_full_attention_v1`, with formula
  `uniform_kv_cache_v1` and method `uniform_full_attention`. It supports uniform
  full-attention `llama` metadata, defaults missing KV-head and head-width fields
  with visible provenance, and supports F16 (default) and F32 only. Quantized KV
  caches and special attention layouts produce explicit unavailability, not
  guessed values.
- Checked `u64` byte arithmetic, non-negative effective budgets, configurable host
  and device reserves, and conservative product defaults of 1,536 MiB host and
  1,024 MiB device reserve.
- Two qualified scenarios: `cpu_weights_and_kv` and
  `device_weights_cpu_kv`. Outcomes are only
  `estimated_components_within_budget`,
  `estimated_components_exceed_budget`, and `insufficient_data`. The plan never
  emits `model_fits`, `launchable`, `supported`, or `safe_to_run`.
- A budget-limited context upper bound with declared-context, cap, and
  runtime-overhead caveats.
- Deterministic unit and CLI integration tests for arithmetic, reserves,
  boundaries, overflow, profiles, missing metadata, cache types, proxy labeling,
  device uncertainty, JSON round-trips, malformed files, output purity, and exit
  codes.
- [Memory planner behavior](MEMORY_PLANNER.md), [plan schema](PLAN_SCHEMA.md),
  dated [KV-cache planning research](research/2026-10-21-kv-cache-planning.md), and
  the [Stage 03 report](../reports/stages/stage-03-memory-planner.md).
- Updated README, changelog, policy, status, architecture, testing, catalog,
  engine-support, contributor, and user documentation. `ghostram.doctor/1` and
  `ghostram.models.scan/1` are unchanged.

**Acceptance evidence:** formatting, Clippy with warnings denied, all workspace
tests, the locked workspace build, documentation checks, commit-range checks,
diff checks, and local identity checks passed. Exact commands, counts, and outcomes
are recorded in the Stage 03 report. All planner evidence uses synthetic GGUF
metadata and synthetic resource snapshots. No real model, engine, GPU allocation,
inference run, or benchmark was used, and no model or engine was installed or
downloaded.

**Gate:** the maintainer reviews the Stage 03 report and accepts the stage.
Acceptance does not authorize Stage 04, a push, tag, GitHub Release, or promotion
to `main`; each remains a separate decision.

**Scope reconciliation:** the earlier Stage 03 wording named candidate lists,
backend-overhead estimates, quantized-KV cases, unsafe-candidate refusal, and
calibration against Stage 05 measurements. The maintainer-authorized Stage 03 scope
was narrowed to a deterministic planning core with no runtime. Candidate
generation, quantized-KV modeling, launch refusal/override policy, runtime pressure
control, and calibration are deferred to later stages and are not hidden incomplete
requirements. The planner reports `insufficient_data` rather than pretending to
refuse a launch, because no launch path exists in Stage 03.

**Non-goals:** model loading, tensor validation, engine compatibility, launch or
refusal, process supervision, GPU layer prediction, partial offload, runtime
overhead measurement, quantized KV, shard grouping, persistence, network access,
GUI, and benchmarks.

## Stage 04 — Engine adapter and supervised runtime

**State:** Next (blocked). It must not begin until Stage 03 is accepted.

**Goal:** launch, observe, and stop a real engine safely.

**Candidate evaluation plan (not started):**

The first adapter remains official llama.cpp. Stage 04 planning must compare these
configurations without installing or running any of them during Stage 03:

1. **CPU baseline** on the primary laptop. It is the reference path and works
   without a GPU runtime. Capture build identity, thread/context settings, memory,
   and a small bounded smoke run only after approval.
2. **Official Linux x64 CUDA 12.8 build** as the Pascal candidate. Verify the
   artifact's actual device code, bundled runtime libraries, driver requirements,
   and compute-capability 6.1 behavior at the artifact level. The label alone is
   not evidence.
3. **Official Linux x64 Vulkan build** as the independent GPU candidate. Verify
   explicit selection of the NVIDIA GTX 1050 Ti rather than the Intel iGPU, then
   compare startup, correctness, and memory behavior with CUDA.
4. **A source build targeting `sm_61` with a CUDA 12.x toolkit**, only if a later
   approved stage installs the required toolchain. It is a comparison candidate,
   not a default. CUDA 13.x is not assumed to build Pascal device code.

Do not install a toolkit or engine, download a model, run inference, run a
benchmark, or change drivers as part of Stage 04 planning. Each candidate requires
an explicit build identity, checksum/provenance, runtime-dependency check, device
visibility check, and a separately approved smoke test.

**Scope:**

- Define the engine adapter contract and the build, installation, and capability
  model.
- Implement the official llama.cpp adapter via a child process and HTTP API.
- Launch `llama-server` on loopback with an API key, explicit model/flags, and
  captured logs; wait for health; enforce startup and idle timeouts.
- Stop the process group on request, timeout, crash, or resource pressure.
- Record resolved arguments, build identity, installation details, runtime
  context, and exit status.
- Add a fake engine for deterministic tests.
- Select and pin the first engine build only after checking its build identity,
  checksum, runtime dependencies, device visibility, and launch behavior.
- Perform the first engine installation and smoke test on the laptop only with
  separate maintainer approval and a small model. Not part of Stage 03 planning.
- Keep the MVP detection-first: do not bundle engine binaries. A later voluntary
  download into a managed directory needs consent, checksums, and license review.
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
- Run a small CPU and GPU matrix (with maintainer approval and small models).
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
- Define engine/runtime acquisition and version pinning for the selected MVP
  model: detect external installations first; do not bundle binaries by default.
- Produce a release candidate, changelog, compatibility statement, and known
  limitations; do not publish without separate maintainer approval for the tag
  and GitHub Release.
- Update status, roadmap, and Stage 08 report.

**Acceptance:** a clean-machine install and uninstall works; no development
toolchain is required; artifacts and checksums are verified; release notes are
accurate; the maintainer approves promotion.

## Later / unscheduled

- Windows support and CUDA/Vulkan packaging.
- Secondary GTX 1070 validation.
- Additional engine adapters (only with demonstrated user value).
- PrismML-specific packings and drafters as an optional adapter.
- Model download management with explicit consent and checksums.
- Advanced search, quality guards, and cross-machine transfer.
- Performance improvements that require upstream engine changes (out of scope
  unless explicitly approved).

## Roadmap rules

- Do not start a stage before its dependencies are accepted.
- Do not implement GUI work before the CLI/services it needs are stable.
- Do not add crates or abstractions without a boundary and a test seam.
- Update this file at the end of every stage with actual state, not intentions.
