# Engine and environment landscape — 2026-10-20

**Research date:** 2026-10-20 (UTC).
**Status:** Stage 00 evidence. This is a snapshot, not a permanent support
statement. External projects change; local observations are specific to the
development laptop.
**Related:** [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md), [ADR-0003](../adr/0003-engine-integration-strategy.md), [Stage 00 report](../../reports/stages/stage-00-technical-reconnaissance.md).

## 1. Scope and method

The research asked:

1. What hardware and software does the primary laptop actually expose?
2. Which inference engine and integration boundary are appropriate for a
   memory-constrained Pascal laptop?
3. Which backend/build combinations are expected to work, which are ruled out,
   and which require an experiment?
4. What evidence exists for low-bit formats and a fork such as PrismML?
5. What can be measured through documented interfaces without linking engine
   libraries?

Evidence sources:

- read-only local commands (OS, CPU, memory, GPU, Vulkan, storage, tool versions);
- upstream source and documentation fetched from GitHub;
- release metadata from GitHub;
- NVIDIA CUDA release notes and GPU tables;
- PrismML documentation and fork source/release metadata;
- upstream issues and pull requests, treated as reports rather than proof.

Confidence labels follow [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md#1-confidence-labels):
**verified locally**, **verified upstream**, **reported**, **hypothesis**, and
**unknown**.

No engine was installed, no model was downloaded, and no inference or benchmark
was run. A source-level conclusion is not a hardware validation.

## 2. Repository starting state

Observed before Stage 00 changes:

- Git repository at `/myfiles/r31nu11/Work/GitHub/GhostRAM`.
- Branch `dev`, tracking `origin/dev`; `main` also at the same initial commit.
- Initial commit `bdfced9` (`Initial commit`) containing `LICENSE` (Apache-2.0)
  and a one-line `README.md`.
- Remote `git@github.com:r31nu11/GhostRAM.git`.
- No tags, no releases, no local Git identity configured.
- No Rust workspace, source code, models, benchmarks, or engine binaries.
- Working tree clean before Stage 00 work.

This matters because there was no prior architecture to preserve and no hidden
application behavior to reverse-engineer. The Stage 00 deliverable is therefore a
contract and evidence base, not a migration.

## 3. Host environment

All values below were observed with read-only commands on 2026-10-20. They are
not guarantees; a rolling distribution, driver update, or desktop session can
change them.

| Item | Observation | Confidence |
| --- | --- | --- |
| OS | CachyOS Linux (rolling), glibc 2.44 | Verified locally |
| Kernel | 7.2.11-1-cachyos | Verified locally |
| Desktop | KDE Wayland, rendered by Intel UHD 630 | Verified locally |
| CPU | Intel Core i7-8750H, 6 cores / 12 threads | Verified locally |
| CPU features | AVX2, FMA, F16C (among others) | Verified locally |
| CPU governor | `powersave` at inspection | Verified locally |
| RAM | 7.4 GiB total; roughly 3.2–3.5 GiB `MemAvailable` at idle | Verified locally |
| Swap | 7.4 GiB zram with `zstd`; about 1.3 MiB used; no disk swap | Verified locally |
| GPU | NVIDIA GeForce GTX 1050 Ti Mobile (GP107M), 4096 MiB | Verified locally |
| Compute capability | 6.1 (Pascal) | Verified locally / NVIDIA table |
| GPU driver | NVIDIA 580.178.04 | Verified locally |
| CUDA runtime reported by driver | Maximum 13.0 | Verified locally (`nvidia-smi`) |
| Vulkan loader | 1.4.363 | Verified locally |
| Vulkan devices | NVIDIA GTX 1050 Ti (Vulkan 1.4.312) and Intel UHD 630 (Mesa 26.2.4) | Verified locally |
| Storage | 41 GiB free on `/`; 773 GiB free on `/myfiles` | Verified locally |

Implications:

- The desktop consumes a substantial part of system RAM. `MemAvailable`, not
  `MemFree`, is the relevant budget input.
- zram can absorb pressure without disk I/O; swap growth is still a pressure
  signal, but it does not mean the same thing as disk swapping.
- The NVIDIA GPU is idle because the desktop uses the Intel iGPU. This is good for
  compute experiments, but a backend that selects the "first" Vulkan device may
  choose Intel by mistake.
- The 4 GiB VRAM and ~7.4 GiB RAM budget is tight enough that model size, context,
  KV cache, and desktop memory must be planned together.

## 4. Tool availability

| Tool | State | Relevance |
| --- | --- | --- |
| `rustc`, `cargo` | 1.94.1, stable x86_64 | Future workspace. |
| `git` | 2.56.0 | Repository workflow. |
| `gcc` | 16.2.1 | Potential native builds. |
| `clang` | 23.1.3 | Potential native builds. |
| `python3` | 3.14.8 | Repository checks and development reference tools. |
| `jq`, `rg`, `fd` | Present | Inspection/research. |
| `vulkaninfo`, `clinfo` | Present | Device diagnostics. |
| `cmake`, `ninja` | Missing | Required for a local llama.cpp build. |
| `nvcc`, CUDA toolkit | Missing | Required for a local CUDA build. |
| `libcudart`, cuBLAS | Not found in the dynamic linker cache | Prebuilt CUDA binaries may need a runtime asset. |
| llama.cpp | Not installed | No engine validation possible. |
| Ollama | Not installed | Not used in the initial strategy. |
| `gh` | Missing | No GitHub CLI operations. |
| `pip`, `uv` | Missing | No Python package installation performed. |
| Models | None found in inspected common directories | No model validation possible. |

Installing these tools, downloading an engine, or downloading a model requires
explicit approval. Stage 00 did none of it.

## 5. Official llama.cpp

### 5.1 Upstream state at access

- Repository: `ggml-org/llama.cpp`, MIT license.
- Version in `CMakeLists.txt`: `0.7.0`.
- Latest release tag: `v0.7.0`, published 2026-10-15.
- Latest nightly build observed: `b11756`, published 2026-10-20 13:48 UTC.
- Release assets for Linux x64 included CPU, CUDA 12.8, CUDA 13.4, and Vulkan
  archives. Approximate observed sizes: CPU 21 MiB, CUDA 12.8 engine 169 MiB,
  CUDA 12.8 cudart 566 MiB, Vulkan 33 MiB.
- The release documentation distinguishes release tags from nightly/development
  builds and describes installer/package-manager consumption. A tag is not
  automatically a GitHub Release object.

Confidence: verified upstream from release metadata, source, and documentation at
the access date. Asset sizes are approximate metadata observations, not local file
measurements.

### 5.2 Backends

The upstream README lists CPU, CUDA, HIP, Vulkan, SYCL, Metal, OpenCL, CANN, MUSA,
WebGPU, RPC, and others. This project's initial interest is CPU, CUDA, and Vulkan
on Linux x64. Backend presence in the family does not mean a particular artifact
contains it or that it works on this GPU.

### 5.3 Process and HTTP surfaces

Verified upstream in the server documentation:

- `llama-server` exposes a loopback HTTP API, including `/health`, `/props`,
  `/v1/models`, `/completion`, `/v1/chat/completions`, and optional `/metrics`.
- `/health` returns 503 while loading and 200 when ready.
- `/props` exposes build info, model path, slots, default generation settings,
  modalities, and related properties.
- `/v1/models` exposes model metadata such as training context, parameter count,
  size, vocabulary, and architecture/modalities in current builds.
- Completion responses include `timings` with prompt and generation counts, times,
  throughput, and cache hits.
- `/metrics` requires `--metrics` and exposes Prometheus counters/gauges.

These surfaces are sufficient for a first adapter without linking `libllama`.
Their exact fields may vary by build, so probes and parsers must be versioned.

### 5.4 Measurement tool

`llama-bench` supports JSON output (`-o json`) and records:

- build commit/number;
- CPU/GPU information and backend list;
- model identity, size, and parameter count;
- tested parameters;
- per-repetition samples;
- average and standard deviation.

The documentation states that `llama-bench` does not include tokenization or
sampling time. Server `timings` include a different boundary. A report must say
which tool produced a number.

### 5.5 Configuration surface

Relevant flags verified in upstream documentation:

- GPU offload: `-ngl`, `--n-gpu-layers` (exact, `auto`, `all`).
- Context: `-c`, `--ctx-size`.
- Batch: `-b`, `--batch-size`; `-ub`, `--ubatch-size`.
- KV cache: `-ctk`, `-ctv` with `f32`, `f16`, `bf16`, `q8_0`, `q4_0`, `q4_1`,
  `iq4_nl`, `q5_0`, `q5_1`.
- Flash Attention: `-fa on|off|auto`.
- Device selection: `-dev`, `--device`; `--list-devices`.
- Load mode: `--load-mode` (`auto`, `none`, `mmap`, `mlock`, `mmap+mlock`, `dio`).
- Memory fitting: `--fit on|off` (server default `on`), `--fit-target` (default
  1024 MiB per device), `--fit-ctx` (default 4096).
- Server: `--parallel`, `--metrics`, `--slots`, `--props`, `--api-key`, host/port.
- Speculative decoding: draft model and `--spec-type` options; build-dependent.

`--fit` is a reproducibility hazard. It may adjust unset arguments, and `/props`
may expose the resolved context but not every resolved value. A benchmark must
either set the relevant arguments explicitly and disable fit, or record the
resolved values and treat them as part of the configuration.

## 6. CUDA on Pascal (compute capability 6.1)

### 6.1 Toolchain constraint

The CUDA 13.0 release notes state that Maxwell, Pascal, and Volta are
feature-complete and that offline compilation and library support for these
architectures were removed in CUDA Toolkit 13.0. The 12.x series continues to
support building for them.

llama.cpp's CUDA CMake logic appends `50-virtual`, `61-virtual`, and `70-virtual`
only when the toolkit version is less than 13. It also builds newer architectures
depending on the toolkit version.

Consequences:

- A CUDA 13.x toolkit cannot compile new device code targeting this GPU. The
  distro's `cuda 13.4.3` package is therefore unsuitable for a Pascal-targeted
  CUDA build.
- This is a toolchain constraint, not a blanket runtime verdict on every
  CUDA 13.x-labeled artifact. A prebuilt's actual GPU code, bundled libraries,
  driver requirements, and execution environment must be inspected and tested.
- The official CUDA 12.8 Linux asset is expected to contain `61-virtual` PTX and
  rely on driver JIT. This is a source-level expectation, not a local execution.
- A native build with `-DCMAKE_CUDA_ARCHITECTURES=61` is expected to work with
  CUDA 12.x and may avoid JIT, but it was not attempted because `cmake`, `ninja`,
  `nvcc`, and a CUDA 12.x toolkit are not installed.
- `libcudart` and cuBLAS were not present locally. The official CUDA archive ships
  a separate cudart asset; the engine archive alone cannot be assumed to be
  self-contained.

### 6.2 Local GPU and driver

- GPU: GTX 1050 Ti Mobile, GP107M, 4 GiB, compute capability 6.1.
- Driver: 580.178.04; `nvidia-smi` reports maximum CUDA runtime 13.0.
- The GPU is idle during desktop use because rendering is on the Intel iGPU.

Driver runtime compatibility does not prove that a CUDA 12.8 binary will
initialize or that PTX JIT will succeed. That requires an experiment.

### 6.3 Upstream Pascal reports

These are reported, not locally reproduced:

- Open issue 30334: Pascal MMVQ fusion check reportedly misses compute capability
  6.1 on a P40.
- Open issue 30235: a Pascal DP4A MMQ configuration reportedly gives about +12%
  prompt processing on sm_61 with a one-line tile change.
- Open issue 30524: PTX `61-virtual` code JIT-compiled on Turing has a host/device
  batch-limit mismatch. Different device, but it shows why PTX-only builds need
  targeted validation.
- Issue 25909: official Windows CUDA prebuilt binaries reportedly lacked sm_61;
  the reporter used Vulkan instead.

These reports are reasons to test carefully, not proof about this laptop.

## 7. Vulkan

Observed locally:

- Vulkan loader 1.4.363.
- Intel UHD 630 (Mesa 26.2.4) and NVIDIA GTX 1050 Ti (NVIDIA 580.178.04,
  Vulkan 1.4.312) are both visible.
- The desktop renders on Intel, leaving NVIDIA available for compute.
- Vulkan requires the system loader and driver, not a CUDA toolkit.

Implications:

- Explicit NVIDIA device selection is required. A "first device" default is not
  safe on this machine.
- Upstream has pre-Turing Vulkan code paths (for example, a PR changing NVIDIA
  `MUL_MAT_ID` row geometry except for pre-Turing devices), which indicates
  support but not performance.
- No Vulkan token-generation benchmark was run. CUDA-vs-Vulkan on Pascal remains
  open.

## 8. PrismML fork

### 8.1 Relationship to upstream

- Repository: `PrismML-Eng/llama.cpp`, branch `prism`, MIT license.
- It is a fork that tracks upstream and adds low-bit formats and runtime features;
  it is not a separate inference architecture.
- At access, the GitHub compare API reported 171 commits ahead of upstream master
  and 0 behind.
- Latest release observed: `prism-b10787-73bfe6c`, dated 2026-10-19.
- Linux x64 assets included CPU, CUDA 12.4, CUDA 12.8, CUDA 13.3, Vulkan, ROCm 7.2,
  and SYCL variants.

### 8.2 Formats and features

- `Q2_0` (Bonsai 2) support, including the Hadamard activation runtime, was merged
  upstream in PR #29600 and is available in upstream builds from `b11555` onward.
- `PQ2_0` is the fork's ggml type (group-128, id 142).
- `PTQ1_0` is under upstream review in PR #29672 and remains fork-specific while
  open.
- MTP and DFlash2 drafters require fork binaries according to PrismML docs.
- Vendor backend documentation marks Vulkan `PQ2_0`/`PTQ1_0` as implemented but
  device/driver-dependent. It describes source-level status, not a guarantee for
  every device.
- A commit dated 2026-10-19 adds a CUDA `PQ2_0` mat-vec path for 1–4 columns on
  Pascal (sm_61). This is evidence of active Pascal work, not a local benchmark.

### 8.3 Assessment

Potential value:

- Low-bit Bonsai models may fit a 4 GiB/8 GiB machine better than standard
  quantizations.
- Pascal-specific CUDA work and speculative decoding may be practically useful.
- The fork reuses the llama.cpp process/HTTP surface, so adapter cost is moderate.

Reasons to defer:

- It is a fast-moving fork with model-specific formats and kernels.
- Its benefit on this laptop is unproven.
- Vendor performance claims are not independent evidence.
- Supporting it expands the compatibility and test matrix.
- The MVP can prove the control-plane value with official llama.cpp first.

Recommendation: official llama.cpp first; PrismML as an optional later adapter or
build profile only when a concrete need is demonstrated and measured.

## 9. GGUF and model identity

GGUF is the initial model format. It is an extensible single-file format with typed
metadata and tensor data, specified by ggml. A parser can read the header and
metadata without loading tensor payloads.

For the MVP, extract:

- GGUF version, architecture, model name;
- file type/quantization, parameter count where present;
- context length, block/layer count, attention head counts, embedding length;
- tokenizer model and relevant metadata;
- split/shard metadata and sidecars;
- file size and SHA-256.

Treat model files as untrusted: bounded reads, checked lengths, no panics, no
execution. Parser candidates included `gguf` 0.1.x, `gguf-rs` 0.1.x, a minimal
in-tree reader, and `gguf-py`'s dump tool as a development reference. At the time
of this 2026-10-20 note, no parser had been selected or audited. Stage 02 later
selected a narrow in-tree read-only reader and did not audit the external crates;
the later scope and evidence are in the
[GGUF catalog format research](2026-10-21-gguf-catalog-format.md). The Stage 02
scope also superseded this note's shard grouping and mandatory-hash assumptions.

## 10. Cross-cutting findings and risks

1. **Capability is per build and runtime context.** Family documentation cannot
   answer whether this executable on this backend, device, and driver enables a
   capability or accepts a model type.
2. **CUDA 13.x is not a Pascal build toolchain.** The version is recent but
   cannot target compute capability 6.1; any CUDA 13.x-labeled prebuilt still
   requires artifact-level inspection and testing.
3. **Two Vulkan devices.** Device selection is part of correctness, not a detail.
4. **4 GiB VRAM is the binding constraint.** A model that loads may still be
   unusably slow or force the host into zram.
5. **`--fit` changes the experiment.** Reproducible comparisons need explicit
   values or resolved-value capture.
6. **Server timings and `llama-bench` measure different boundaries.** Do not mix
   them.
7. **PrismML is promising but optional.** It should not delay the official
   adapter.
8. **No hardware evidence exists yet.** All backend performance statements are
   hypotheses until measured.

## 11. Open questions

1. Does the official CUDA 12.8 Linux binary initialize on compute capability 6.1
   with driver 580.178.04, and does it use PTX JIT or a real 6.1 binary?
2. Is a native CUDA 12.8 `-DCMAKE_CUDA_ARCHITECTURES=61` build materially faster
   or more stable than the prebuilt `61-virtual` path?
3. Which Vulkan device selector reliably chooses the NVIDIA GPU, and does the
   backend perform correctly on pre-Turing hardware?
4. Is Vulkan faster, slower, or equivalent to CUDA 12.8 for prompt processing and
   generation on this GPU?
5. Which model sizes and context lengths remain usable within the RAM/VRAM budget?
6. Does PrismML provide a measurable benefit on this laptop, and do its Pascal
   PQ2_0 and speculative paths work outside the vendor's tested hardware?
7. Which GGUF parser is safe, maintained, and accurate enough?
8. How should `--fit`-resolved values be captured across builds?
9. What usability threshold separates "loads" from "practically usable"?
10. Which engine build should be the first pinned baseline?

## 12. Sources

All accessed 2026-10-20 unless noted. Confidence in the linked claim is noted in
the relevant section above.

### Official llama.cpp

- Repository and README: <https://github.com/ggml-org/llama.cpp>
- Build guide: <https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md>
- Server documentation: <https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md>
- Benchmark tool: <https://github.com/ggml-org/llama.cpp/blob/master/tools/llama-bench/README.md>
- Release process: <https://github.com/ggml-org/llama.cpp/blob/master/docs/release.md>
- CUDA architecture logic: <https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-cuda/CMakeLists.txt>
- Release workflow: <https://github.com/ggml-org/llama.cpp/blob/master/.github/workflows/release.yml>
- Nightly b11756: <https://github.com/ggml-org/llama.cpp/releases/tag/b11756>
- v0.7.0 tag: <https://github.com/ggml-org/llama.cpp/releases/tag/v0.7.0>
- GGUF specification: <https://github.com/ggml-org/ggml/blob/master/docs/gguf.md>

### NVIDIA

- CUDA 13.0 release notes: <https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html>
- CUDA 12.8 release notes: <https://docs.nvidia.com/cuda/archive/12.8.0/cuda-toolkit-release-notes/index.html>
- CUDA GPU compute capability table: <https://developer.nvidia.com/cuda-gpus>

### PrismML

- Fork: <https://github.com/PrismML-Eng/llama.cpp>
- llama.cpp documentation: <https://docs.prismml.com/run/llamacpp>
- Formats and runtime support: <https://docs.prismml.com/download/formats>
- Backend support: <https://github.com/PrismML-Eng/Bonsai-demo/blob/main/BACKEND-SUPPORT.md>
- Release: <https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10787-73bfe6c>
- Pascal PQ2_0 commit: <https://github.com/PrismML-Eng/llama.cpp/commit/483bdaba>
- Upstream Bonsai 2 merge: <https://github.com/ggml-org/llama.cpp/pull/29600>
- Upstream PTQ1_0 PR: <https://github.com/ggml-org/llama.cpp/pull/29672>

### Reports and issues

- Issue 25909 (Windows CUDA prebuilt / sm_61): <https://github.com/ggml-org/llama.cpp/issues/25909>
- Issue 29174: <https://github.com/ggml-org/llama.cpp/issues/29174>
- Issue 30235 (Pascal DP4A MMQ): <https://github.com/ggml-org/llama.cpp/issues/30235>
- Issue 30334 (Pascal MMVQ fusion): <https://github.com/ggml-org/llama.cpp/issues/30334>
- Issue 30524 (PTX 61-virtual on Turing): <https://github.com/ggml-org/llama.cpp/issues/30524>
- Vulkan pre-Turing path: <https://github.com/ggml-org/llama.cpp/pull/29274>

### Future UI

- Iced 0.14.0: <https://github.com/iced-rs/iced/releases/tag/0.14.0>

## 13. What this research does not establish

- That any engine build works on this laptop.
- That CUDA or Vulkan is faster for this GPU.
- That the official CUDA 12.8 binary runs on compute capability 6.1.
- That a native Pascal build is stable or faster.
- That PrismML formats or drafters work here.
- That a particular model fits within memory.
- That a benchmark result exists. None does.
