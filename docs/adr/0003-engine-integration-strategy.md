# ADR-0003: Process-based engine integration, official llama.cpp first

**Status:** Proposed
**Date:** 2026-10-20
**Deciders:** human repository owner, project engineers
**Related:** [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md), [research notes](../research/2026-10-20-engine-landscape.md)

## Context

GhostRAM is a control plane, not an inference engine. It needs an engine that
runs on the primary laptop: an NVIDIA GTX 1050 Ti Mobile with 4 GiB VRAM, compute
capability 6.1 (Pascal), 7.4 GiB system RAM, and CachyOS Linux. The NVIDIA driver
is 580.178.04; Vulkan is available; no CUDA toolkit or engine is installed.

Evidence gathered in Stage 00:

- Official llama.cpp provides CPU, CUDA, and Vulkan backends, a server HTTP API,
  and `llama-bench` with JSON output.
- CUDA 13.0 removed offline compilation and library support for Pascal. A CUDA
  12.x build is required to target compute capability 6.1; CUDA 13.4 is
  unsuitable.
- The official CUDA 12.8 Linux archive is expected to include `61-virtual` PTX,
  but this was not executed locally.
- Vulkan does not require the CUDA toolkit, but this laptop exposes both Intel UHD
  630 and NVIDIA GTX 1050 Ti devices, so explicit selection matters.
- PrismML maintains a llama.cpp fork with additional low-bit formats and
  speculative-decoding work. It is not a separate inference architecture, and its
  benefit on this GPU is unproven.

Linking `libllama` into GhostRAM would create ABI coupling, complicate crash
isolation, and make build/backend variation harder to represent. A custom engine
is far outside the MVP and would delay the control-plane work.

## Decision

1. Integrate engines as **external child processes** through documented
   interfaces. The initial adapter uses `llama-server` over loopback HTTP and
   `llama-bench` for controlled throughput.
2. Use **official llama.cpp first**. Do not build the MVP around a fork.
3. Preserve **CPU** as the baseline and fallback backend.
4. Treat CUDA and Vulkan as separate build/backend configurations with separate
   capability probes and measurements.
5. On Pascal hardware, use a **CUDA 12.x** build or **Vulkan**. Do not use CUDA
   13.x builds for compute capability 6.1.
6. Treat **PrismML as an optional later adapter/profile**, not the default. Add it
   only when a concrete model or capability need is demonstrated and measured.
7. Probe capabilities per installed build. Do not infer support from the engine
   family, a flag in `--help`, or a vendor matrix alone.
8. Do not link engine libraries into the control plane in the first version.

## Consequences

Positive:

- Engine crashes, OOM, and driver resets do not take down the control plane.
- The same adapter shape can support official llama.cpp and later a PrismML build.
- Users can use an existing engine build instead of a GhostRAM-specific build.
- HTTP and process boundaries are observable, testable, and replaceable.

Negative:

- Process startup, health checks, log capture, and cleanup add complexity.
- The HTTP API and flags vary by build and may change upstream.
- Some internal metrics or optimizations are unavailable through the public
  interface.
- `llama-server` and `llama-bench` measure different things; reports must explain
  the difference.

Follow-up:

- Define the adapter contract and fake engine in Stage 04.
- Validate the official CUDA 12.8 and Vulkan builds only with owner authorization.
- Revisit FFI only if a documented capability cannot be achieved through the
  process boundary.
- Keep engine build identity and capability evidence with every run.

## Alternatives considered

- **Link `libllama` via FFI:** tighter integration and potentially lower overhead,
  but fragile ABI, harder crash isolation, and a larger build matrix.
- **Write a Rust inference engine:** far outside scope, duplicates mature work,
  and cannot be validated in the MVP.
- **Use Ollama or another bundled runtime first:** convenient, but hides engine
  build details and limits backend/flag control that this project needs.
- **Adopt PrismML first:** potentially useful low-bit formats on 4 GiB, but a
  fast-moving fork with unproven value on Pascal; it would expand the test burden
  before the control plane works.
- **Use CUDA 13.x on Pascal:** rejected by upstream support removal; it cannot
  target compute capability 6.1.

## References

- [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md)
- [research notes](../research/2026-10-20-engine-landscape.md)
- [NVIDIA CUDA 13.0 release notes](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html)
- [llama.cpp build guide](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md)
- [llama.cpp server API](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
