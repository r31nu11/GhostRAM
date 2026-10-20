# Quick start

**Status: not available.** GhostRAM has no executable, no release, and no
working command-line interface. The flow below is a product contract for a future
release, not instructions that work today.

Do not install an engine, download a model, or change system settings solely
because this page describes a planned feature. Those actions require explicit
user authorization. Within project development, engine or model installation also
requires maintainer approval.

## 1. Planned first-run flow

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

## 2. What the first release will not do

- It will not install GPU drivers or a CUDA toolkit.
- It will not silently download models or engines.
- It will not guarantee that a model fits or that a configuration is fast.
- It will not support every engine, backend, or operating system.
- It will not replace llama.cpp or another inference engine.
- It will not require root or a system-wide daemon.

## 3. Current alternatives

There is nothing to quick-start in this repository. The documentation is the
deliverable of Stage 00. If you are looking for a working local inference tool
today, use the engine project directly and follow its own documentation; GhostRAM
does not yet wrap or replace it.

## 4. Related documents

- [Installation contract](INSTALL.md)
- [Project status](PROJECT_STATUS.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Benchmarking methodology](BENCHMARKING.md)
