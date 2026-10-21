# Quick start

**Status: no release; source diagnostics, a model scanner, and a memory planner are
available.** There is no installer or supported release. A developer can build the
source and run the read-only `ghostram doctor`, `ghostram models scan`, and
`ghostram models plan` commands. The multi-step inference flow below remains a
product contract for a future release.

Do not install an engine, download a model, or change system settings solely
because this page describes a planned feature. Those actions require explicit
user authorization. Within project development, engine or model installation also
requires maintainer approval.

## 1. Source-development diagnostic check

This is not an end-user installation. It requires a Rust toolchain and operates on
the source checkout:

```sh
cargo build --workspace --locked
cargo run -p ghostram-cli -- doctor
cargo run -p ghostram-cli -- doctor --json
```

`doctor` is read-only. It reports OS/kernel, CPU, RAM, swap/zram, storage, NVIDIA
GPU/VRAM/driver, Vulkan devices, relevant tools, and discovered llama.cpp
executables. Missing optional tools, GPUs, Vulkan devices, and engines are reported
as unavailable or absent; the command still exits successfully when it can
generate a report. The JSON contract is in [DOCTOR_SCHEMA.md](DOCTOR_SCHEMA.md).

It does not install an engine, download a model, launch inference, benchmark
anything, or change system settings.

## 2. Source-development model catalog check

The source scanner requires a directory that you explicitly select. It has no
default model path and never follows symlinks:

```sh
cargo run -p ghostram-cli -- models scan /path/to/models
cargo run -p ghostram-cli -- models scan /path/to/models --recursive
cargo run -p ghostram-cli -- models scan /path/to/models --json
cargo run -p ghostram-cli -- models scan /path/to/models --recursive --hash
```

The default is non-recursive. `--json` writes the `ghostram.models.scan/1`
envelope. Parsing is bounded and read-only; it extracts compact GGUF metadata and
never emits tokenizer values, chat templates, or tensor bytes. Without `--hash`,
tensor payloads are not read. `--hash` explicitly reads every complete candidate
file to compute SHA-256, so it can take a long time on large models. There is no
cache, database, `models list`, shard grouping, or engine cross-check. The JSON
contract is in [MODELS_SCHEMA.md](MODELS_SCHEMA.md) and the parser/scan limits are
in [GGUF_CATALOG.md](GGUF_CATALOG.md).

## 3. Source-development memory plan check

The planner requires one explicitly selected GGUF file. It does not scan a
directory, choose a model automatically, or hash the file:

```sh
cargo run -p ghostram-cli -- models plan /path/to/model.gguf
cargo run -p ghostram-cli -- models plan /path/to/model.gguf --json
cargo run -p ghostram-cli -- models plan /path/to/model.gguf \
  --context 4096 --sequences 1 --kv-cache f16
```

Useful options include `--context`, `--sequences`, `--kv-cache f16|f32`,
`--ram-reserve-mib`, `--vram-reserve-mib`, and `--device`. Defaults are visible in
the output. The command reads host memory and NVIDIA device information, but never
launches or probes an inference engine.

The plan reports only modeled weights (a low-confidence file-size proxy) and a
narrow uniform full-attention `llama` KV estimate. Outcomes are qualified as
`estimated_components_within_budget`, `estimated_components_exceed_budget`, or
`insufficient_data`. A within-budget result does **not** mean that the model will
load or run: runtime/backend overhead is unknown, and launch success is not
verified. The JSON contract is in [PLAN_SCHEMA.md](PLAN_SCHEMA.md); behavior,
formula limits, and evidence classes are in
[MEMORY_PLANNER.md](MEMORY_PLANNER.md).

## 4. Planned first-run flow

When an alpha exists, a new user should be able to:

1. **Install GhostRAM** through a documented, checksummed channel. No Rust
   toolchain or compiler should be required. See [INSTALL.md](INSTALL.md).
2. **Run a read-only environment check.** The application reports OS/kernel, CPU,
   RAM, swap/zram, GPU, VRAM, driver, Vulkan/OpenCL devices, storage, and whether
   an inference engine is discoverable. It must not launch inference yet.
3. **Point GhostRAM at a model directory.** The user chooses the directory; the
   application scans it without modifying files.
4. **Inspect discovered models.** The source scanner shows architecture, common
   context/layer/attention fields, a quantization hint, file size, filename hints,
   and an optional hash. It does not group shards or prove engine compatibility.
5. **Review a plan.** The source planner now shows modeled weights, a narrow KV
   estimate, evidence classes, assumptions, reserves, and two qualified scenarios.
   A plan is not a candidate optimization or a launch decision. Candidate search
   remains planned.
6. **Run one bounded experiment.** With the user's confirmation, GhostRAM starts
   the selected engine on loopback, waits for readiness, sends a bounded request,
   samples memory and pressure, and stops the process.
7. **Read the evidence.** The result contains measured throughput, time to first
   token where available, peak memory, swap/pressure indicators, load time, and
   failures, with full engine/model/hardware provenance.
8. **Compare candidates.** Results are presented with variance and a Pareto view,
   not a single unqualified "best" number.

Steps 1-2 are represented only by source commands and the read-only diagnostics;
steps 3-4 have a source scanner but no release UX, persistent catalog, or shard
grouping. Step 5 has a source planner for modeled weights and one narrow KV profile,
but no engine allocation validation, candidate search, or launch decision. Steps
6-8 are planned. None of the full flow has been implemented or validated end to
end.

## 5. What the first release will not do

- It will not install GPU drivers or a CUDA toolkit.
- It will not silently download models or engines.
- It will not guarantee that a model fits or that a configuration is fast.
- It will not support every engine, backend, or operating system.
- It will not replace llama.cpp or another inference engine.
- It will not require root or a system-wide daemon.

## 6. Current alternatives

The source-built read-only commands are `ghostram doctor`,
`ghostram models scan`, and `ghostram models plan`. There is no engine runtime,
benchmark, tuner, or GUI to quick-start. The model catalog is a metadata scanner,
not a model loader. The planner is a pure estimate, not a launcher or safety
proof. If you are looking for a working local inference tool today, use the engine
project directly and follow its own documentation; GhostRAM does not yet wrap or
replace it.

## 7. Related documents

- [Installation contract](INSTALL.md)
- [Project status](PROJECT_STATUS.md)
- [Doctor schema](DOCTOR_SCHEMA.md)
- [GGUF catalog behavior](GGUF_CATALOG.md)
- [Model scan schema](MODELS_SCHEMA.md)
- [Memory planner behavior](MEMORY_PLANNER.md)
- [Plan schema](PLAN_SCHEMA.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Benchmarking methodology](BENCHMARKING.md)
