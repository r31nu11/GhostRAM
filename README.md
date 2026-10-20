# GhostRAM

GhostRAM is a planned open-source Rust application for intelligent local LLM
inference configuration, resource management, benchmarking, and optimization on
constrained hardware.

> **Status: Stage 01 diagnostic implementation complete; pending formal
> acceptance.** A source-buildable `ghostram` CLI now provides a read-only
> `doctor` command with human-readable and versioned JSON output. There is still
> no release artifact, installer, engine integration, model catalog, inference,
> benchmark result, or GUI. No engine or model was installed or downloaded for
> Stage 01. Do not treat source-development commands as an end-user installation.

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

Claims will be tied to recorded experiments, engine build identities, hardware,
and configuration. Unknowns are documented as unknowns.

## Primary development target

The immediate development and validation laptop, as observed on 2026-10-20:

| Component | Observation |
| --- | --- |
| OS | CachyOS (rolling), kernel 7.2.11-1-cachyos |
| CPU | Intel Core i7-8750H, 6 cores / 12 threads, AVX2/FMA/F16C |
| RAM | 7.4 GiB total; approximately 3.2-3.5 GiB available at idle inspection |
| Swap | 7.4 GiB zram (zstd), not a disk swap partition; usage varies with the desktop session |
| GPU | NVIDIA GeForce GTX 1050 Ti Mobile (GP107M), 4096 MiB VRAM |
| GPU driver | NVIDIA 580.178.04; compute capability 6.1 (Pascal) |
| Display GPU | Intel UHD Graphics 630 via KDE Wayland; the NVIDIA GPU was idle |
| Vulkan | Loader 1.4.363; NVIDIA device exposes Vulkan 1.4.312 |
| Storage | Approximately 41 GiB free on `/` at the Stage 01 run; the repository resides on `/myfiles` |

A secondary validation machine (GTX 1070, 16 GiB RAM, Intel i5-8600) is planned
but was not accessed during Stage 00 or Stage 01. Its exact configuration must be
verified before any claim is made about it.

## Current state

| Area | State |
| --- | --- |
| Repository workflow | `main`/`dev` policy, commit checks, documentation checks, and Rust CI jobs implemented |
| Project contract | Charter, MVP, non-goals, roadmap, release policy documented and accepted |
| Architecture decisions | ADR-0001 through ADR-0005 accepted on 2026-10-20 |
| Diagnostic CLI | `ghostram doctor` and `ghostram doctor --json` implemented; source-buildable, not released |
| Hardware probe | Read-only Linux probes for OS/kernel, CPU, memory, swap/zram, storage, NVIDIA GPU/driver, Vulkan, tools, and engine paths implemented |
| Engine discovery | `llama-server`, `llama-bench`, and `llama-cli` path/version/device discovery implemented; **no engine installed or verified** |
| Engine integration | Process adapter and inference launch **not implemented** |
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
- [Doctor schema](docs/DOCTOR_SCHEMA.md) — versioned diagnostic JSON contract.
- [Engine support](docs/ENGINE_SUPPORT.md) — integration matrix and confidence levels.
- [Benchmarking](docs/BENCHMARKING.md) — methodology and reporting contract.
- [Research notes](docs/research/2026-10-20-engine-landscape.md) — sources and evidence.
- [Stage 00 report](reports/stages/stage-00-technical-reconnaissance.md) — accepted foundation and validation.
- [Stage 01 report](reports/stages/stage-01-minimal-rust-workspace.md) — diagnostic implementation and validation.

For users and future contributors:

- [Installation contract](docs/INSTALL.md) — release installation requirements and source-development notes.
- [Quick start](docs/QUICK_START.md) — source diagnostic commands and the planned first-run flow.
- [Troubleshooting](docs/TROUBLESHOOTING.md) — current diagnostics, environment findings, and limitations.
- [Uninstall contract](docs/UNINSTALL.md) — planned data locations and removal rules.
- [Contributing](docs/CONTRIBUTING.md) — branches, commits, reviews, and docs.
- [Testing](docs/TESTING.md) — current Rust checks and future test strategy.
- [Releasing](docs/RELEASING.md) — versioning and release checklist.
- [Architecture decision records](docs/adr/README.md) — accepted decisions and the ADR process.

## Build from source for development

There is no release or installer. The source workspace contains exactly
`ghostram-core`, `ghostram-probe`, and `ghostram-cli`; building it requires a Rust
toolchain. This is a development workflow, not an end-user installation.

```sh
cargo build --workspace --locked
cargo run -p ghostram-cli -- doctor
cargo run -p ghostram-cli -- doctor --json
```

The `doctor` command is read-only. It reports missing optional tools, GPUs,
Vulkan devices, and engines as absent or unavailable rather than failing. The
JSON schema is documented in [docs/DOCTOR_SCHEMA.md](docs/DOCTOR_SCHEMA.md).

Run the repository checks from the root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
```

The local commit hook is opt-in and is not installed globally:

```sh
git config core.hooksPath .githooks
```

Development and intermediate commits happen on `dev` or on temporary branches
based on `dev`. `main` receives only maintainer-accepted, working/stable
checkpoints. Promotion from `dev` to `main` requires maintainer acceptance and
applicable validation. Pushes are not automatic. Tags and GitHub Releases require
separate explicit maintainer approval; a documentation-only stage is not a stable
application release. The project maintainer is `r31nu11`, and commits use the
maintainer's repository-local Git identity.

## Safety

GhostRAM treats the host as a shared, finite resource. Stage 01 implements a
read-only doctor: it does not require root, does not install or download anything,
does not launch inference, and kills bounded subprocesses on timeout. Planned
safeguards for later stages include explicit memory budgets, one heavy inference
process at a time, loopback-only engine servers, no silent downloads, and
termination of child processes on resource pressure. Those later safeguards are
design requirements, not implemented behavior.

## License

GhostRAM is licensed under the [Apache License 2.0](LICENSE). External inference
engines, model files, and GPU drivers remain under their own licenses. Release
artifacts will include third-party notices when they bundle external components.
