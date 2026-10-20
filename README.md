# GhostRAM

GhostRAM is a planned open-source Rust application for intelligent local LLM
inference configuration, resource management, benchmarking, and optimization on
constrained hardware.

> **Status: Stage 00 foundation only.** There is no runnable GhostRAM binary,
> no inference engine integration, and no GUI yet. The repository currently
> contains the project charter, architecture, research, workflow, and quality
> checks that later stages will implement against. Do not follow installation or
> usage instructions from other projects as if they were GhostRAM instructions.

## What GhostRAM is intended to do

GhostRAM will help a user answer practical questions such as:

- What CPU, RAM, GPU, VRAM, driver, and compute backends are available?
- Which local GGUF models can run, and with what context, offload, and cache settings?
- Which candidate configuration is likely to fit within a safe memory budget?
- Which configuration actually performs best for a chosen goal on this machine?
- How do results compare across engine builds, backends, and model revisions?
- How can repeated experiments avoid destabilizing the host system?

The initial product will supervise existing inference engines. It will not
implement a complete LLM inference engine or custom GPU kernels.

## What GhostRAM does not promise

GhostRAM does **not** claim that it can:

- make an arbitrary model faster,
- fit an arbitrary model into limited VRAM,
- outperform existing tools without reproducible measurements,
- replace llama.cpp or another inference engine,
- automatically produce a good result on every hardware combination.

Claims will be tied to recorded experiments, engine versions, hardware, and
configuration. Unknowns are documented as unknowns.

## Primary development target

The immediate development and validation laptop, as observed on 2026-10-20:

| Component | Observation |
| --- | --- |
| OS | CachyOS Linux (rolling), kernel 7.2.11-1-cachyos |
| CPU | Intel Core i7-8750H, 6 cores / 12 threads, AVX2/FMA/F16C |
| RAM | 7.4 GiB total; approximately 3.2-3.5 GiB available at idle inspection |
| Swap | 7.4 GiB zram (zstd), not a disk swap partition |
| GPU | NVIDIA GeForce GTX 1050 Ti Mobile (GP107M), 4096 MiB VRAM |
| GPU driver | NVIDIA 580.178.04; compute capability 6.1 (Pascal) |
| Display GPU | Intel UHD Graphics 630 via KDE Wayland; the NVIDIA GPU was idle |
| Vulkan | Loader 1.4.363; NVIDIA device exposes Vulkan 1.4.312 |
| Storage | 41 GiB free on `/`; 773 GiB free on `/myfiles` |

A secondary validation machine (GTX 1070, 16 GiB RAM, Intel i5-8600) is planned
but was not accessed during Stage 00. Its exact configuration must be verified
before any claim is made about it.

## Current state

| Area | State |
| --- | --- |
| Repository workflow | `main`/`dev` policy and commit checks documented and implemented |
| Project contract | Charter, MVP, non-goals, roadmap, release policy documented |
| Technical research | llama.cpp, CUDA/Pascal, Vulkan, and PrismML findings recorded with sources |
| Engine integration | Recommended strategy documented; **not implemented or installed** |
| Hardware probe | Requirements documented; **not implemented** |
| GGUF model catalog | Requirements documented; **not implemented** |
| Planner, tuner, runtime, store | Architecture boundaries documented; **not implemented** |
| Iced UI | Planned; **not started** |
| Benchmarks | Methodology documented; **no benchmarks executed and no results exist** |

## Documentation

Start here:

- [Project charter](docs/PROJECT_CHARTER.md) — mission, MVP, non-goals, success criteria.
- [Project status](docs/PROJECT_STATUS.md) — current stage, branch, and blockers.
- [Roadmap](docs/ROADMAP.md) — dependency-ordered stages.
- [Architecture](docs/ARCHITECTURE.md) — components, boundaries, and data flow.
- [Engine support](docs/ENGINE_SUPPORT.md) — integration matrix and confidence levels.
- [Benchmarking](docs/BENCHMARKING.md) — methodology and reporting contract.
- [Research notes](docs/research/2026-10-20-engine-landscape.md) — sources and evidence.
- [Stage 00 report](reports/stages/stage-00-technical-reconnaissance.md) — starting state and validation.

For users and future contributors:

- [Installation contract](docs/INSTALL.md) — planned installation requirements, not yet available.
- [Quick start](docs/QUICK_START.md) — planned first-run flow, not yet available.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — current environment findings and known limitations.
- [Uninstall contract](docs/UNINSTALL.md) — planned data locations and removal rules.
- [Contributing](docs/CONTRIBUTING.md) — branches, commits, reviews, and docs.
- [Testing](docs/TESTING.md) — current checks and future test strategy.
- [Releasing](docs/RELEASING.md) — versioning and release checklist.
- [Architecture decision records](docs/adr/README.md) — accepted and proposed decisions.

## Development in Stage 00

There is no Cargo workspace yet. The only executable checks are documentation
and commit-message checks:

```sh
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
```

The local commit hook is opt-in and is not installed globally:

```sh
git config core.hooksPath .githooks
```

Development and intermediate commits happen on `dev` or on temporary branches
based on `dev`. `main` receives only owner-accepted, working/stable checkpoints.
Promotion from `dev` to `main` requires owner acceptance and applicable
validation. Pushes are not automatic. Tags and GitHub Releases require separate
explicit owner approval; a documentation-only stage is not a stable application
release. The repository owner is `r31nu11`, and commits use the owner's local Git
identity.

## Safety

GhostRAM will treat the host as a shared, finite resource. Planned safeguards
include explicit memory budgets, one heavy inference process at a time,
loopback-only engine servers, no silent downloads, no root-required operations,
and termination of child processes on timeout or resource pressure. These are
design requirements; they are not yet implemented.

## License

GhostRAM is licensed under the [Apache License 2.0](LICENSE). External inference
engines, model files, and GPU drivers remain under their own licenses. Release
artifacts will include third-party notices when they bundle external components.
