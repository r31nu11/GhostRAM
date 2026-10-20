# Architecture

**Status:** Proposed for Stage 00; pending formal acceptance. No component below
is implemented.
**Related decisions:** [ADR-0002](adr/0002-modular-monolith-and-crate-split.md),
[ADR-0003](adr/0003-engine-integration-strategy.md),
[ADR-0004](adr/0004-measurement-provenance.md),
[ADR-0005](adr/0005-pinned-engine-builds.md).

## 1. Scope and stance

GhostRAM is a control plane for external inference engines. It discovers
hardware and models, estimates candidate configurations, launches and supervises
engine processes, measures their behavior, and stores evidence. It does not
perform tensor computation itself.

The architecture is a **modular monolith** at first: one Cargo workspace, a small
number of crates, explicit module boundaries, and process-level isolation for
engines. Crates are split only when a boundary has a real dependency, test, or
ownership reason. The ten-crate list in the project brief is a target map, not a
requirement to create ten empty crates.

## 2. System context

```text
┌──────────────────────────────────────────────────────────────────────┐
│ GhostRAM control plane                                               │
│                                                                      │
│  CLI (ghostram)                 Iced UI (later)                      │
│        │                              │                              │
│        └──────────────┬───────────────┘                              │
│                       ▼                                              │
│              application services                                    │
│   doctor / catalog / plan / run / bench / history                    │
│        │              │             │              │                 │
│        ▼              ▼             ▼              ▼                 │
│  hardware probe   model catalog   planner      run store             │
│        │              │             │              │                 │
│        └──────────────┴──────┬──────┴──────────────┘                 │
│                              ▼                                       │
│                     engine adapter contract                          │
│                              │                                       │
└──────────────────────────────┼───────────────────────────────────────┘
                               │ child process + HTTP/CLI
                               ▼
                 ┌─────────────────────────────┐
                 │ inference engine build      │
                 │ llama.cpp / PrismML fork    │
                 │ CPU / CUDA / Vulkan device  │
                 └─────────────────────────────┘
                               │
                               ▼
                 GPU driver, system RAM, swap, disk
```

The control plane and the inference plane have separate failure domains. A
segfault, CUDA OOM, or driver reset in an engine process must not corrupt the
planner or the stored history.

## 3. Proposed responsibilities

| Component | Responsibility | Initial form |
| --- | --- | --- |
| `ghostram-core` | Domain types, units, IDs, configuration intent, errors, capability and measurement contracts, versioned serialization envelopes. | Crate |
| `ghostram-probe` | Read-only Linux hardware discovery: OS, CPU, memory, swap, GPU, driver, Vulkan/OpenCL devices, storage, tool availability. | Crate |
| `ghostram-models` | Local GGUF discovery, metadata parsing, identity/hash, shard grouping. | Module first; crate when parser lands |
| `ghostram-engine` | Engine adapter contract, build discovery, capability probes, argument mapping, child-process lifecycle, HTTP client. | Crate; runtime initially inside |
| `ghostram-planner` | Memory estimates, candidate generation, safety budgets, Pareto inputs. | Module first; crate when estimates are non-trivial |
| `ghostram-tuner` | Bounded search, experiment scheduling, evaluation against goals. | Later crate |
| `ghostram-runtime` | Long-lived process supervision if it outgrows the engine adapter. | Later split from engine |
| `ghostram-store` | Persistence of runs, measurements, model/engine fingerprints. | Module behind a trait; crate when SQLite lands |
| `ghostram-cli` | Thin command-line surface, JSON/human output, exit codes. | Crate |
| `ghostram-ui` | Iced desktop application. | Later crate |

### Initial crate recommendation

Stage 01 should create only:

1. `ghostram-core`
2. `ghostram-probe`
3. `ghostram-cli`

`ghostram-engine` follows in Stage 04 when there is a process to supervise.
`ghostram-models` follows in Stage 02 when a real parser exists. `planner`,
`tuner`, `store`, and `ui` remain modules or future crates until their APIs are
stable. This avoids ten crates with placeholder code and circular dependency
pressure.

Dependency direction:

```text
cli ──► probe
 │       │
 ├──────► engine ──► core
 │          │
 ├──────► models ───┘
 │          │
 └──────► planner ──► core
            │
            ▼
          store
```

Rules:

- `core` depends on no OS-specific crate and no engine.
- `probe` may depend on OS APIs and `core`, never on the UI or engine.
- `models` may depend on a parser and `core`, never on `probe`.
- `engine` may depend on `core`, process/HTTP libraries, and filesystem paths;
  it must not depend on `models` or `planner` types beyond `core` contracts.
- `planner` is pure and deterministic where possible; it takes metadata and
  hardware snapshots, returns candidates and estimates.
- `cli` and `ui` are adapters; they do not contain domain rules.
- No crate reads global configuration implicitly. Configuration is passed in.

## 4. Domain model

The exact Rust definitions will be finalized in Stage 01. The following contracts
are required.

### 4.1 Units and identity

- `Bytes(u64)`, `Mib(u32)`, `Tokens(u32)`, `TokensPerSecond(f64)`,
  `Milliseconds(u64)`. No untyped integer memory fields in public APIs.
- `ModelId`: stable identity from content hash plus a display name; a path alone
  is not an identity.
- `EngineBuildId`: stable identity derived from the executable SHA-256 and
  immutable build metadata. The executable path and runtime probe output are not
  inputs; moving an executable or probing it on another device does not create a
  different build.
- `HardwareFingerprint`: normalized OS/kernel, CPU model, RAM, GPU model, VRAM,
  driver, and Vulkan device identity.
- `RunId`: unique, sortable identifier generated by GhostRAM, not by the engine.

### 4.2 Configuration

- `ConfigIntent`: engine-independent user intent (goal, context tokens, offload
  policy, KV precision preference, batch preference, thread policy, memory
  budget, load mode).
- `ResolvedConfig`: the exact values GhostRAM will pass to a specific build,
  including defaults it filled in and engine-resolved values such as a
  `--fit`-adjusted context. This is the object benchmarks compare.
- `EngineArg`: a typed name/value pair with provenance (`user`, `default`,
  `adapter`, `engine-resolved`). Arguments are passed as an array to the child
  process; never assembled into a shell string.
- Unknown engine-specific options may be retained as opaque key/value pairs with
  provenance, but the adapter must validate them and record them.

### 4.3 Engine builds, installations, and capabilities

- `EngineFamily`: for example `LlamaCpp` or `LlamaCppPrismMl`.
- `EngineBuild`: immutable build identity (`EngineBuildId`), engine family,
  executable SHA-256, known version/tag/commit and artifact metadata, target
  platform, build configuration, and known bundled dependency identity. It does
  not include the executable path or runtime probe results.
- `EngineInstallation`: executable path, discovery method, local installation
  details, and observation time. An installation may change without changing the
  build identity.
- `CapabilityObservation`: `{ capability, result: supported | unsupported |
  unknown, evidence: Text, source: ProbeSource, observed_at: Timestamp, build_id:
  EngineBuildId, runtime_context: RuntimeContext }`. `RuntimeContext` records the
  backend, selected device, driver/runtime version, and available dependency
  evidence.
- Family-level documentation must never be converted into a build-level "yes".
  A missing or failed probe yields `unknown`, not `supported`. An observation
  from one backend, device, or driver does not automatically apply to another.

Examples of capability questions:

- Does this build expose `/props`, `/metrics`, `/slots`?
- Does it accept `-ctk q8_0` and actually use it on this backend?
- Does it support `--fit`, speculative decoding, or `PQ2_0` model loading?
- Which devices are visible, and which one is selected by default?
- Does the build contain code for compute capability 6.1?

### 4.4 Prediction and measurement

- `Estimate<T>`: value or range, method, assumptions, confidence, created_at,
  calibration dataset (if any).
- `Measurement<T>`: observed value, unit, sampling method/interval, run ID,
  engine build, device, and timestamp.
- There is no implicit conversion from `Estimate<T>` to `Measurement<T>`.
- Reports must label every performance number as `estimated`, `measured`, or
  `unknown`. A measured value without a run manifest is invalid.

### 4.5 Errors

- `core` exposes typed errors for invalid configuration, unsupported capability,
  budget violation, engine startup failure, engine protocol failure, and
  persistence failure.
- `anyhow` may be used only at the CLI/UI boundary for context; library APIs
  return typed errors.
- Errors carry enough context to be actionable but never include secrets or
  model contents.

## 5. Required boundaries

### 5.1 Predicted memory vs measured memory

Predictions use model metadata and a documented formula. Measurements use
external observation of the actual process and device. They live in different
fields, different JSON objects, and different report sections. A prediction may be
compared with a measurement only in a labeled calibration record. A prediction
never overwrites a measurement.

### 5.2 Engine-independent settings vs engine-specific options

Users express intent (context, offload, cache precision, batch, threads). The
adapter maps intent to concrete flags for a build and returns the resolved
configuration. Engine-specific flags are never leaked into the planner's domain
model. If an intent cannot be represented by a build, the adapter returns
`Unsupported`, not an approximate flag with different semantics.

### 5.3 Estimated performance vs experimentally verified performance

Estimates may rank candidates before a run. Only measured results can be used to
claim that one configuration is faster, to populate a Pareto frontier, or to
recommend a default. The tuner must keep the two datasets separate and must not
train on unverified estimates as if they were observations.

### 5.4 Engine-family claims vs observed build capabilities

The support matrix describes what is believed about engine families and
backends. Runtime capability observations describe the actual executable, in a
specific backend, device, driver, and dependency context, at a specific time. The
observation wins for that context. A user may install a llama.cpp build without
CUDA, with CUDA 13.x, or with a fork that rejects a model type; GhostRAM must
detect that build's behavior rather than assuming the family's documentation.
An observation must not be generalized to another build or runtime context
without evidence.

### 5.5 Application control plane vs inference engine

GhostRAM owns configuration, process lifecycle, scheduling, and persistence. The
engine owns model loading, tokenization, inference, and its own memory
management. Communication is through documented process/HTTP interfaces. GhostRAM
does not read engine internal memory, patch engine libraries, or assume that an
engine's logs are stable machine-readable data without a versioned parser.

## 6. Engine adapter contract

Illustrative shape, not final Rust code:

```rust
trait EngineAdapter {
    fn family(&self) -> EngineFamily;
    fn discover(&self) -> Result<Vec<EngineInstallation>, EngineError>;
    fn probe_build(&self, installation: &EngineInstallation)
        -> Result<EngineBuild, EngineError>;
    fn capabilities(
        &self,
        build: &EngineBuild,
        installation: &EngineInstallation,
        context: &RuntimeContext,
    ) -> CapabilitySet;
    fn resolve(&self, intent: &ConfigIntent, build: &EngineBuild)
        -> Result<ResolvedConfig, EngineError>;
    fn start(&self, config: &ResolvedConfig, limits: &ResourceLimits)
        -> Result<EngineSession, EngineError>;
    fn health(&self, session: &EngineSession) -> Result<Health, EngineError>;
    fn run(&self, session: &EngineSession, workload: &Workload)
        -> Result<MeasurementSet, EngineError>;
    fn stop(&self, session: EngineSession) -> Result<StopReport, EngineError>;
}
```

Discovery yields installation records; probing an installation yields build
identity; capability observations link the build to a runtime context. These are
separate records so that a path change or a device probe cannot silently redefine
the build.

`EngineSession` owns a process handle, a log sink, a loopback base URL, and a
shutdown token. `Drop` must not be the only shutdown mechanism; explicit stop and
a supervisor are required. The adapter must support cancellation and timeout.

Initial adapter: official llama.cpp.

- Discovery: configurable paths, `PATH`, and user-specified executable.
- Probe: `--version`, `--list-devices`, `--help`; launch-time `/health`,
  `/props`, `/v1/models`; optional `/metrics`.
- Launch: `llama-server` bound to `127.0.0.1`, random port or configured port,
  random API key, `--parallel 1` for benchmarks, explicit model and flags.
- Data: streaming `/v1/chat/completions` or `/completion` for TTFT and timings;
  `llama-bench -o json` for controlled throughput.
- Logs: captured to a run-scoped file; parsed only for known build ranges, with
  raw text retained.

PrismML adapter is the same family with a different build profile. It should
reuse the same process and HTTP code; only capability detection, supported model
types, and optional speculative-decoding flags differ.

## 7. Runtime and process model

```text
start request
   │
   ├─ preflight: executable, model, budgets, port, config validation
   ├─ spawn child in its own process group; capture stdout/stderr
   ├─ wait for /health 200 or startup log signal, with timeout
   ├─ record resolved config, build identity, installation, runtime context,
   │  devices, and initial memory
   ├─ run workload(s) or wait for user
   └─ stop: request graceful shutdown, wait, then kill process group
```

Requirements:

- One heavy engine process at a time by default.
- Use a process group (Unix) so descendants are terminated together.
- Never pass user strings through a shell.
- Sanitize the environment; do not inherit secrets unnecessarily.
- Bind to loopback only; use an API key even on loopback.
- Enforce startup, request, and idle timeouts.
- Persist logs and exit status even on failure.
- On abnormal exit, mark the run failed and preserve diagnostics.
- Do not auto-restart a crashed benchmark run unless the experiment explicitly
  allows retries; retries are recorded.

## 8. Data flow and persistence

```text
probe ──► hardware snapshot ──┐
                              ├─► plan ──► candidates ──► run ──► measurements
model scan ──► metadata ──────┘                                      │
                                                                     ▼
                                                               run store
                                                                     │
                                                              JSON + Markdown
```

Engine discovery produces installation records, probing produces build identity,
and capability observations are timestamped against a runtime context. These are
separate from hardware and model identity and are not collapsed into a single
fingerprint.

Persistence requirements:

- Versioned JSON envelopes with a schema version.
- UTC RFC 3339 timestamps.
- Content hashes for models and engine binaries.
- Build identity, installation records, and capability observations stored
  separately and linked explicitly.
- Raw samples retained alongside summaries.
- Atomic writes; never corrupt an existing run.
- No secrets or API keys in stored records.
- A run record is immutable after completion; corrections are new records with
  a link to the superseded run.
- SQLite is a candidate for querying history, but not required for the first
  JSON-based implementation. The store trait hides the choice.

## 9. Safety governor

The planner and runtime share a resource policy:

- Preflight uses `MemAvailable` (not `MemFree`), free VRAM, and current pressure.
- Default reserves are configurable. Initial proposal: 1024 MiB VRAM margin
  (consistent with llama.cpp's `--fit-target` default) and at least 1536 MiB
  system RAM margin. These numbers are hypotheses on an 8 GiB laptop and must be
  calibrated.
- If predicted usage exceeds the budget, refuse unless the user explicitly
  overrides with a recorded acknowledgement.
- During a run, sample memory/VRAM/PSI/swap and stop on sustained pressure or a
  hard threshold. Avoid sampling so frequently that it distorts the measurement.
- Do not rely on zram as free memory. Swap growth is a pressure signal.
- Do not change CPU governor, GPU clocks, or driver settings automatically.
- Warn when running on battery; benchmark results on battery are marked
  incomparable unless power state is recorded and controlled.
- Never delete model files or user data as cleanup.

## 10. UI architecture (later)

Iced 0.14.x is the current planned toolkit as of Stage 00, but it is not a
dependency yet. The UI should:

- consume the same application services as the CLI;
- keep long-running probes/runs off the render thread;
- show explicit `estimated`/`measured` labels and confidence;
- expose cancellation and engine shutdown;
- avoid duplicating engine argument logic;
- be built only after the CLI and service contracts are stable.

The primary laptop runs KDE Wayland with both Intel and NVIDIA Vulkan devices;
the UI must not assume that the first Vulkan adapter is the discrete GPU.

## 11. Testing seams

- `probe`: parsers for captured `/proc`, `/sys`, `nvidia-smi`, `vulkaninfo`, and
  version outputs; fixtures for missing/odd hardware.
- `models`: malformed/truncated GGUF fixtures; shard grouping; bounded reads.
- `planner`: property tests for monotonicity and overflow; golden estimates.
- `engine`: fake adapter and fake HTTP server; timeout/crash/cleanup tests;
  argument-array snapshots.
- `store`: round-trip and schema-version tests.
- `cli`: JSON schema and exit-code tests.
- Hardware: manual smoke tests on the laptop and later the GTX 1070; never
  pretend CI covers GPU behavior.

## 12. Open architecture questions

1. Does the first adapter need `llama-bench` at all, or can server timings cover
   the MVP? (Proposed: both, because they answer different questions.)
2. Should the engine adapter own process supervision permanently, or split into
   `runtime` once multiple engines exist?
3. Which GGUF parser is safe and maintained enough for untrusted files?
4. How should `--fit`-resolved values be recovered reliably across builds?
5. What is the minimum usable schema for history, and when does SQLite become
   justified?
6. How should the UI display uncertainty without overwhelming users?
7. Which packaging format best supports Vulkan/CUDA runtime dependencies on
   Linux?
