# Quick start

**Status: no release; source diagnostic available.** There is no installer or
supported release. A developer can build the source and run the read-only
`ghostram doctor` command. The multi-step inference flow below remains a product
contract for a future release.

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

## 2. Planned first-run flow

When an alpha exists, a new user should be able to:

1. **Install GhostRAM** through a documented, checksummed channel. No Rust
   toolchain or compiler should be required. See [INSTALL.md](INSTALL.md).
2. **Run a read-only environment check.** The application reports OS/kernel, CPU,
   RAM, swap/zram, GPU, VRAM, driver, Vulkan/OpenCL devices, storage, and whether
   an inference engine is discoverable. It must not launch inference yet.
3. **Point GhostRAM at model directories.** The user chooses directories; the
   application scans them without modifying files.
4. **Inspect discovered models.** The catalog shows architecture, parameter count,
   context limit, quantization, file size, hash, and shard information.
5. **Review a plan.** GhostRAM shows candidate configurations with explicit
   estimates, assumptions, and safety budgets. Estimates are labeled as
   estimates.
6. **Run one bounded experiment.** With the user's confirmation, GhostRAM starts
   the selected engine on loopback, waits for readiness, sends a bounded request,
   samples memory and pressure, and stops the process.
7. **Read the evidence.** The result contains measured throughput, time to first
   token where available, peak memory, swap/pressure indicators, load time, and
   failures, with full engine/model/hardware provenance.
8. **Compare candidates.** Results are presented with variance and a Pareto view,
   not a single unqualified "best" number.

These steps are planned. None has been implemented or validated end to end.

## 3. What the first release will not do

- It will not install GPU drivers or a CUDA toolkit.
- It will not silently download models or engines.
- It will not guarantee that a model fits or that a configuration is fast.
- It will not support every engine, backend, or operating system.
- It will not replace llama.cpp or another inference engine.
- It will not require root or a system-wide daemon.

## 4. Current alternatives

The only working GhostRAM command is the source-built read-only `doctor` check.
There is no model catalog, planner, engine runtime, benchmark, or GUI to
quick-start. If you are looking for a working local inference tool today, use the
engine project directly and follow its own documentation; GhostRAM does not yet
wrap or replace it.

## 5. Related documents

- [Installation contract](INSTALL.md)
- [Project status](PROJECT_STATUS.md)
- [Doctor schema](DOCTOR_SCHEMA.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Benchmarking methodology](BENCHMARKING.md)
