# GhostRAM project charter

**Status:** Stage 00 proposal, awaiting human owner review.
**Last substantive revision:** 2026-10-20.
**Owner:** the human repository owner. AI agents act as technical leads and
repository engineers under that authority.

## 1. Purpose

GhostRAM exists to make local LLM inference on constrained hardware more
predictable. It will discover what a machine actually has, estimate which
configurations are plausible, run bounded experiments through existing inference
engines, and record reproducible evidence about what worked.

The project is not a new inference engine. It is a control plane, planner,
experiment runner, and knowledge base around engines such as llama.cpp.

## 2. Problem and intended users

Local inference is full of hidden constraints:

- A model file may fit on disk but not in RAM plus VRAM.
- A GPU may be present but unsupported by a particular engine build.
- A configuration may load successfully and then be unusably slow.
- Context size, KV-cache precision, batch size, GPU-layer count, and backend
  choice interact in ways that are difficult to predict.
- Benchmark results are often reported without engine version, driver, model
  hash, variance, or failure data, making comparisons unreliable.
- On an 8 GiB laptop with a 4 GiB Pascal GPU, an overly ambitious launch can
  destabilize the desktop or trigger the OOM killer.

Primary users:

1. People running models on older or memory-constrained laptops and desktops.
2. Users who already have GGUF models and an inference engine but do not know
   which settings are worth trying.
3. Contributors who need a reproducible way to compare engine builds and
   backends on real hardware.

Secondary users:

1. Maintainers of engine forks who want an external measurement harness.
2. Users on secondary machines (for example the planned GTX 1070 validation
   machine) who need a configuration that transfers with explicit caveats.

## 3. Product vision

GhostRAM should eventually:

- detect CPU, RAM, swap, GPU, VRAM, driver, and supported compute backends;
- discover local GGUF models and inspect metadata without modifying them;
- estimate memory requirements and identify plausible configurations;
- integrate with external engines through explicit, capability-aware adapters;
- supervise engine processes and stop them safely;
- benchmark real configurations and compare measured results;
- optimize for declared goals such as generation speed, balanced behavior,
  longer context, or the largest practically usable model;
- record reproducible results and learn from previous measurements;
- provide a native Rust desktop interface, initially planned with Iced; and
- provide a CLI for diagnostics, benchmarking, and automation.

Linux is the first target. Windows is a later target. The exact supported
distributions, driver versions, and packaging formats are not yet decided.

## 4. Value proposition

GhostRAM must provide value beyond launching an engine with different flags. The
intended differentiators are:

1. **Capability-aware integration.** The application distinguishes an engine
   family from a particular installed build and checks what that build and device
   actually support.
2. **Safety-aware planning.** Candidate configurations are evaluated against
   explicit RAM/VRAM budgets and host-pressure signals, not just "did it load".
3. **Reproducible experiments.** Every result carries model identity, engine
   build, backend, device, driver, configuration, workload, and variance data.
4. **Multi-objective search.** The tuner can compare speed, context length, model
   size, and failure risk instead of optimizing a single number.
5. **Historical reuse.** Measurements from the same hardware and model family can
   warm-start later searches, with uncertainty and transfer limits recorded.
6. **Explicit uncertainty.** Predictions and measurements are separate types;
   estimates are labelled and never presented as observed performance.

These are product hypotheses. They are not proven until the corresponding
experiments in [the research notes](research/2026-10-20-engine-landscape.md) and
future stage reports produce evidence.

## 5. Principles

1. **Measure, then claim.** Performance statements require recorded experiments
   with identifiable hardware and software.
2. **The host is shared.** GhostRAM must leave enough RAM and VRAM for the
   operating system and the user's desktop. A successful load is not enough.
3. **No silent system mutation.** No driver installation, package installation,
   registry change, or system-wide tuning without explicit user action.
4. **Process isolation.** Engines run as child processes behind explicit
   interfaces; GhostRAM does not link its own inference implementation into the
   control plane.
5. **Local first.** No telemetry, cloud dependency, or model upload by default.
   Network access for model or engine downloads must be explicit and auditable.
6. **Reproducibility over convenience.** Pin engine builds, record hashes, and
   keep raw samples even when summaries hide them.
7. **Honest uncertainty.** Unknown combinations are marked unverified. Failed
   experiments are retained, not hidden.
8. **Boring architecture first.** Use the smallest number of crates and
   abstractions that keeps boundaries testable; split later when evidence
   demands it.
9. **Human authority.** The owner approves milestones, hardware validation,
   promotion to `main`, and public releases.

## 6. MVP definition

The first usable alpha is CLI-first. It should:

1. Produce a hardware and environment report for the local machine.
2. Discover installed inference engine builds and report their observed
   capabilities and device visibility.
3. Scan one or more user-specified directories for GGUF models and extract
   identity and architectural metadata without loading tensor data.
4. Estimate memory for a small set of candidate configurations with explicit
   assumptions and uncertainty.
5. Refuse candidates that violate configured RAM/VRAM safety budgets unless the
   user explicitly overrides the refusal.
6. Launch a selected engine build under supervision, wait for readiness, run a
   bounded request, and stop it without leaving orphan processes.
7. Record throughput, time to first token where measurable, peak memory, swap or
   pressure indicators, load time, and failures in machine-readable JSON.
8. Render a human-readable Markdown report from the same data.
9. Compare a bounded set of candidates and present a Pareto frontier rather than
   a single claimed "best" configuration.

The MVP does **not** require a GUI, automatic model downloads, automatic engine
installation, Windows support, or a novel optimizer.

## 7. Explicit non-goals

GhostRAM will not, in the MVP or near term:

- implement a complete LLM inference engine;
- write custom GPU kernels or patch llama.cpp;
- guarantee speedups, fit arbitrary models, or beat other tools without evidence;
- download or redistribute model weights by default;
- modify model files, quantize models, or convert formats;
- install GPU drivers, CUDA toolkits, or system packages automatically;
- run as root or require system-wide services;
- provide a cloud service, telemetry, or remote benchmarking by default;
- support every engine, backend, quantization, or operating system;
- treat a successful process exit as proof of usable performance;
- treat a documented architecture as an experimentally validated one;
- publish releases or tags without human approval.

## 8. Success criteria for the alpha

The alpha is successful when, on the primary laptop and at least one independent
run, it can:

- identify the hardware, driver, engine build, backend, and device without manual
  interpretation;
- catalogue a local GGUF model and report its architecture, parameter count,
  context limit, quantization, and file hash;
- produce a memory estimate whose error against measured peak usage is recorded,
  with the estimate method and assumptions visible;
- run a constrained candidate set on CPU and at least one GPU backend without
  crashing the desktop or leaving orphan processes;
- produce a JSON run record and a Markdown summary that another person can use to
  reproduce the run;
- distinguish a configuration that is merely loadable from one that meets a
  declared usability threshold;
- report failures and variance honestly, including runs that were not attempted.

These criteria require hardware tests that Stage 00 did not perform.

## 9. Constraints and assumptions

- Primary laptop: 4 GiB VRAM, 7.4 GiB RAM, Pascal GTX 1050 Ti Mobile, CachyOS.
- The desktop consumes a substantial fraction of system RAM; available memory at
  inspection was roughly 3.2-3.5 GiB.
- Swap is zram, not disk-backed; swap growth indicates memory pressure but not
  necessarily disk thrashing.
- CUDA 13.x no longer provides offline compilation or library support for Pascal;
  CUDA 12.x is required for a CUDA build targeting compute capability 6.1.
- The distro's current `cuda` package is 13.4.x and is therefore not suitable for
  a Pascal CUDA build.
- Vulkan is available through the installed NVIDIA driver and does not require a
  CUDA toolkit, but the integrated Intel GPU is also visible and must not be
  selected accidentally.
- Engine interfaces and CLI flags change. Capabilities must be probed per build
  and pinned for reproducible tests.
- The secondary GTX 1070 machine is not assumed to be accessible.
- No models or engine binaries are installed in the development environment at
  the end of Stage 00.

## 10. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Pascal CUDA support is frozen and fragile | CUDA path may be slower or buggier than expected | Prefer CUDA 12.8; keep Vulkan and CPU fallbacks; test natively |
| 4 GiB VRAM / 8 GiB RAM is very tight | Few models fit; host may destabilize | Conservative budgets, small models/context first, abort on pressure |
| zram hides memory pressure | A "successful" run may be compressing heavily | Monitor PSI, swap growth, and throughput; define usability thresholds |
| Engine churn | Adapter breakage, unreproducible results | Pin builds, probe capabilities, record raw outputs |
| PrismML fork divergence | Maintenance burden and model-specific behavior | Treat as optional adapter, not default; require measured value |
| Benchmark noise on a laptop | Misleading conclusions | Warmups, repeats, AC power, recorded governor/thermals, variance reporting |
| Model licensing and provenance | Distribution or reproducibility problems | Never redistribute models; record hashes and source paths |
| UI work outruns core | Attractive demo without reliable engine | CLI-first MVP; GUI only after stable services |

## 11. Decision rights and governance

- The human owner approves architecture changes, stage acceptance, promotion to
  `main`, version numbers, tags, and public releases.
- AI agents may propose decisions through ADRs and implement accepted stages.
- A change to an established architectural decision requires a superseding ADR
  with context, reasons, consequences, and migration impact.
- A stage is not accepted because code exists. Its acceptance criteria, tests,
  report, status, and roadmap must agree.
- Destructive or externally visible actions (push, publish, install, driver
  change) require explicit authorization.

## 12. Research questions

The following are hypotheses, not commitments:

1. Does empirical configuration search outperform simple fixed heuristics on
   memory-constrained Pascal hardware?
2. Does the best GPU-layer count change materially with context size and KV-cache
   configuration?
3. Do cache precision, batching, or backend choice produce practically meaningful
   improvements on this laptop?
4. Can historical measurements reduce the number of trials needed for a later
   search without overfitting to one machine?
5. How can GhostRAM distinguish a usable large-model configuration from a
   technically successful but unusably slow, swap-heavy run?

Each question needs a pre-declared experiment, a control, repeats, and a decision
rule. The initial experiment sketches are in
[docs/BENCHMARKING.md](BENCHMARKING.md) and
[docs/research/2026-10-20-engine-landscape.md](research/2026-10-20-engine-landscape.md).

## 13. Scope boundaries

- **GhostRAM controls engines; it does not perform inference.**
- **Predictions are not measurements.** They use separate types and reporting.
- **Engine-family documentation is not build capability.** Every build is probed.
- **Estimated performance is not verified performance.** Only measurements can
  promote a candidate to a verified result.
- **The application control plane is separate from the engine process.** Engine
  crashes must not take down GhostRAM's stored history.

## 14. Open questions for the owner

1. Which engine build should be the first pinned baseline: an official stable
   release or a specific nightly with available Linux CUDA 12.8/Vulkan assets?
2. Is installing an inference engine and a small GGUF model authorized for the
   next stage, and on which storage path?
3. Should GhostRAM eventually bundle an engine/runtime or only detect externally
   installed engines?
4. What usability threshold defines "practically usable" for generation speed
   and context on the 4 GiB laptop?
5. Which packaging format is preferred for Linux: tarball, AppImage, Flatpak, or
   distribution packages?
6. When should the secondary GTX 1070 machine become part of acceptance testing?
