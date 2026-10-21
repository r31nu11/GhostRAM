# Engine support and integration matrix

**Research date:** 2026-10-20 (UTC); model-format review updated 2026-10-21.
**Status:** Stage 00 research accepted; Stage 01 accepts read-only discovery only;
Stage 02 adds a bounded read-only GGUF catalog (pending formal acceptance). No
engine combination below has been executed by GhostRAM on the primary laptop.
"Expected" does not mean "verified", and a discovered executable or a cataloged
GGUF file does not mean "supported".
**Canonical evidence:** [engine research notes](research/2026-10-20-engine-landscape.md)
and [GGUF format research](research/2026-10-21-gguf-catalog-format.md).
**Decision:** [ADR-0003](adr/0003-engine-integration-strategy.md).

## 1. Confidence labels

| Label | Meaning |
| --- | --- |
| **Verified locally** | Observed directly in this environment with a read-only command. |
| **Verified upstream** | Confirmed in upstream source, official documentation, or release metadata at the access date. |
| **Reported** | Stated by a maintainer, vendor, issue reporter, or secondary source; not independently reproduced here. |
| **Hypothesis** | A reasoned expectation that requires an experiment. |
| **Unknown** | No adequate evidence. Do not present it as supported. |

A row may mix labels; each claim keeps its own label.

## 2. Recommended initial strategy

1. **Start with official llama.cpp**, not a fork and not a Rust reimplementation.
2. **Integrate through a child process and HTTP**, not by linking `libllama`
   into GhostRAM in the first version. Use `llama-server` for capability
   introspection and request timings, and `llama-bench` for controlled
   prompt/generation throughput.
3. **Preserve CPU as the baseline and fallback.** CPU results are the reference
   for detecting GPU regressions and the only path that works without a GPU
   runtime.
4. **Treat CUDA and Vulkan as separate build/backend configurations.** They have
   different toolchains, device visibility, failure modes, and performance.
5. **On this Pascal laptop, use a CUDA 12.x toolchain for a CUDA build, or use
   Vulkan.** CUDA 13.x cannot compile new device code for compute capability 6.1.
   A prebuilt labeled CUDA 13.x is a separate case: inspect its actual GPU code,
   bundled libraries, driver requirements, and execution environment before
   treating it as compatible or incompatible.
6. **Evaluate PrismML later as an optional adapter/profile**, not as the default
   engine. It is upstream llama.cpp plus additional low-bit formats and drafters;
   its value on 4 GiB Pascal must be measured.
7. **Pin and record the identity of every engine build.** Do not build tests
   around "latest". Record the executable hash and known provenance, and keep
   installation paths and device observations separate. No specific build is
   pinned in Stage 00 or Stage 01; select the first baseline during Stage 04
   integration after verifying the build, checksum, dependencies, device
   visibility, and launch.
8. **Record capabilities per build and runtime context.** Never infer a build's
   support solely from the engine family, from a flag appearing in `--help`, or
   from a probe on a different backend or device.
9. **Do not install an engine in Stage 01.** The first installation and smoke test
   belong to Stage 04 or later with separate maintainer approval.
10. **Detect external installations in the MVP.** Do not bundle engine binaries. A
    later voluntary download of a verified build into a managed directory requires
    explicit consent, checksums, and a license review.

### 2.1 Stage 01 discovery behavior

Stage 01 searches `PATH` only for `llama-server`, `llama-bench`, and `llama-cli`.
For a found executable it runs bounded read-only queries: `--version` for all three
and `--list-devices` for `llama-server`. It records the path, any version text, any
device list, and a `partial` state with a reason if a probe fails or times out.

Discovery does not verify that an executable runs inference, contains code for
compute capability 6.1, supports a Vulkan device, accepts a model format, or works
with an installed runtime. It does not install, update, or launch an engine. On the
development laptop no llama.cpp executable was installed, so the Stage 01 report
contains an empty engine list. That is an absence observation, not a support
result.

## 3. Compatibility and investigation matrix

Legend: `cc` = compute capability; `ngl` = GPU layers; `KV` = key/value cache;
`FA` = Flash Attention; `ctx` = context size; `fit` = engine memory fitting.

Version `b11756` below is the research snapshot, not a selected baseline. A
CUDA 13.x backend label refers to a CUDA build family; it does not by itself
establish whether a particular prebuilt contains Pascal device code or compatible
runtime libraries. Those are artifact-level properties to inspect and probe, not
assumptions from the label.

| Engine / distribution | Backend | Relevant hardware | Model formats | Configuration capabilities | Integration approach | Measurement approach | Known limitations | Evidence and sources | Open questions |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Official llama.cpp 0.7.0 / nightly b11756 | CPU | Intel i7-8750H, 6C/12T, AVX2 | GGUF | `ngl 0`, threads, ctx, batch/ubatch, KV types, FA, mmap/mlock, fit | Process adapter; `llama-server` + `llama-bench` | `llama-bench -o json`; server `timings`; external memory sampler | CPU is slow for larger models; thermal/power effects; no GPU offload | [README](https://github.com/ggml-org/llama.cpp), [build guide](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md), local toolchain observation | Which CPU thread count is best on 6C/12T under GPU offload? |
| Official llama.cpp 0.7.0 / b11756 | CUDA 12.8, Linux x64 | GTX 1050 Ti Mobile, 4 GiB, cc 6.1 | GGUF | `ngl`, KV types, ctx, batch/ubatch, FA, fit, device selection | Prebuilt binary + CUDA runtime asset or toolkit; process adapter | `llama-bench -o json`; server `timings`; NVML/`nvidia-smi`; `/proc` | Build likely contains `61-virtual` PTX, not native SASS; `libcudart`/cuBLAS not installed locally; no test yet | [CUDA build docs](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md#cuda), [CUDA CMake](https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-cuda/CMakeLists.txt), [b11756 assets](https://github.com/ggml-org/llama.cpp/releases/tag/b11756) | Does the prebuilt run on cc 6.1? Is native `61` build faster or more reliable? |
| Official llama.cpp 0.7.0 / b11756 | CUDA 13.4, Linux x64 | GTX 1050 Ti Mobile, cc 6.1 | GGUF | Unknown; not a Pascal build target | Not recommended; inspect the actual artifact | Artifact-level device-code and library probe | CUDA 13 removed offline compilation/library support for Pascal; llama.cpp CMake omits 50/61/70 for toolkit >= 13. The CUDA 13.4-labeled prebuilt was not inspected, so neither support nor universal incompatibility is assumed. | [CUDA 13.0 release notes](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html), [CUDA CMake](https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-cuda/CMakeLists.txt) | Does this prebuilt contain Pascal device code, or only newer architectures? |
| Official llama.cpp 0.7.0 / b11756 | Vulkan, Linux x64 | GTX 1050 Ti Mobile, 4 GiB; Intel UHD 630 also visible | GGUF | `ngl`, KV types, ctx, batch/ubatch, FA, fit, explicit device selection | Prebuilt Vulkan binary; process adapter | Same as CUDA, plus device-specific selection | Two Vulkan devices; default may choose the Intel iGPU; pre-Turing code paths; performance vs CUDA unknown | Local `vulkaninfo --summary`, [b11756 Vulkan asset](https://github.com/ggml-org/llama.cpp/releases/tag/b11756), [Vulkan build docs](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md#vulkan) | Which device name/ID reliably selects the NVIDIA GPU? Is Vulkan faster than CUDA 12.8 on Pascal? |
| Official llama.cpp | CPU / CUDA 12.x / Vulkan | GTX 1070, 8 GiB, cc 6.1 (secondary machine) | GGUF | Same family controls | Same adapter; separate hardware fingerprint | Same | Machine not accessed; CPU model and exact driver unverified | Project brief only; no local observation | Access date, driver, performance, and whether results transfer from the 1050 Ti |
| PrismML fork (`PrismML-Eng/llama.cpp`, branch `prism`) | CUDA 12.8, Linux x64 | GTX 1050 Ti Mobile, cc 6.1 | GGUF including `Q2_0`, `PQ2_0`, `PTQ1_0`; MTP/DFlash2 drafters | Upstream controls plus fork-specific model types and speculative decoding | Same process/HTTP adapter with a PrismML build profile; optional | Same as CUDA; model-load success/failure; speculative `timings` fields | Fork tracks upstream but adds 171 commits (at access); model-specific; MTP on Pascal uncertain; no independent test | [PrismML README](https://github.com/PrismML-Eng/llama.cpp), [docs](https://docs.prismml.com/run/llamacpp), [formats](https://docs.prismml.com/download/formats), [Pascal commit](https://github.com/PrismML-Eng/llama.cpp/commit/483bdaba) | Does it measurably help on 4 GiB? Do PQ2_0/PTQ1_0 and MTP work on Pascal? |
| PrismML fork | Vulkan, Linux x64 | GTX 1050 Ti Mobile | Same fork formats | Upstream controls plus fork formats | Optional profile; same process adapter | Same | Vendor matrix marks Vulkan PQ2_0/PTQ1_0 as partial/device-dependent; no local test | [Backend support](https://github.com/PrismML-Eng/Bonsai-demo/blob/main/BACKEND-SUPPORT.md), [Prism release](https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10787-73bfe6c) | Do the fork shaders select the NVIDIA device and run correctly? |
| PrismML fork | CUDA 13.3, Linux x64 | GTX 1050 Ti Mobile, cc 6.1 | Fork formats | Unknown; not a Pascal build target | Not recommended; inspect the actual artifact | Artifact-level device-code and library probe | CUDA 13 removed offline compilation/library support for Pascal; fork CMake inherits upstream architecture logic. The CUDA 13.3-labeled prebuilt was not inspected. | [CUDA 13.0 release notes](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html), [Prism CMake](https://github.com/PrismML-Eng/llama.cpp/blob/prism/ggml/src/ggml-cuda/CMakeLists.txt) | Does this prebuilt contain Pascal device code? |
| Official llama.cpp / PrismML | CUDA 12.x or Vulkan | Windows + GTX 1050 Ti / 1070 | GGUF | Same family controls | Later adapter; packaging TBD | Same | Windows support is later; official Windows CUDA prebuilt reportedly lacks sm_61; Vulkan reportedly works | [Issue 25909](https://github.com/ggml-org/llama.cpp/issues/25909) (reported), [build docs](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md) | Is Windows CUDA 12.x packaging viable, or is Vulkan the only practical path? |
| Other engines (Ollama, etc.) | Various | Various | Various | Not investigated | Out of scope for MVP | n/a | Different APIs, bundling, and model stores | None yet | Revisit only with a concrete user need |

### What is *not* claimed

- That the official CUDA 12.8 Linux binary works on this laptop. It is expected,
  not tested.
- That Vulkan works or is faster than CUDA on Pascal. It is available, not
  benchmarked.
- That PrismML improves anything on 4 GiB. Vendor and community reports exist;
  they are not local evidence.
- That the GTX 1070 machine behaves like the laptop.
- That a CUDA 13.x label alone proves a prebuilt cannot run on Pascal. Its actual
  device code, bundled libraries, driver requirements, and environment decide
  that, and the published artifacts were not inspected here.
- That every GGUF quantization or architecture is supported by every build.

## 4. Official llama.cpp details

### 4.1 Current upstream state

At the access date:

- Repository: `ggml-org/llama.cpp`, MIT license.
- Version in `CMakeLists.txt`: `0.7.0`.
- Latest release tag: `v0.7.0`, published 2026-10-15.
- Latest nightly build observed: `b11756`, published 2026-10-20 13:48 UTC.
- Linux x64 assets for `b11756` include CPU, CUDA 12.8, CUDA 13.4, and Vulkan
  archives. Approximate sizes observed from release metadata: CPU 21 MiB,
  CUDA 12.8 engine 169 MiB, CUDA 12.8 cudart 566 MiB, Vulkan 33 MiB.
- The release process document states that releases are not published as GitHub
  Release objects; nightly/development builds are on the releases page, while
  release tags are consumed by the installer, package managers, or source builds.

### 4.2 Backends

The upstream README lists CPU, CUDA, HIP, Vulkan, SYCL, Metal, OpenCL, CANN,
MUSA, WebGPU, RPC, and others. This project's initial interest is CPU, CUDA, and
Vulkan on Linux x64.

### 4.3 Configuration surface relevant to GhostRAM

Verified in the server documentation and `llama-bench` documentation:

- GPU offload: `-ngl`, `--n-gpu-layers` (exact, `auto`, or `all`).
- Context: `-c`, `--ctx-size`.
- Batch: `-b`, `--batch-size`; `-ub`, `--ubatch-size`.
- KV cache: `-ctk`, `-ctv` with `f32`, `f16`, `bf16`, `q8_0`, `q4_0`, `q4_1`,
  `iq4_nl`, `q5_0`, `q5_1`.
- Flash Attention: `-fa on|off|auto`.
- Device selection: `-dev`, `--device`; `--list-devices`.
- Model loading: `--load-mode` (`auto`, `none`, `mmap`, `mlock`, `mmap+mlock`,
  `dio`), lazy mode.
- Memory fitting: `--fit on|off` (server default `on`), `--fit-target` (default
  1024 MiB per device), `--fit-ctx` (default 4096).
- Split mode for multiple GPUs; not relevant to a single laptop GPU.
- Speculative decoding: draft model and `--spec-type` options; build-dependent.
- Server: `--parallel`, `--metrics`, `--slots`, `--props`, `--api-key`, host/port.

Important: `--fit` may adjust unset arguments. For reproducible comparisons,
GhostRAM must either set all relevant values explicitly and disable fit, or record
the engine-resolved values. `/props` exposes the resolved context but not
necessarily the resolved layer count. This is an open measurement problem.

### 4.4 Capability and measurement surfaces

- `--version` prints version and build info.
- `--list-devices` prints available devices and exits.
- `--help` enumerates accepted flags for that build.
- `GET /health`: 503 while loading, 200 when ready.
- `GET /props`: `build_info`, `model_path`, `total_slots`, default generation
  settings, modalities, and other server properties.
- `GET /v1/models`: model metadata including `n_ctx_train`, `n_params`, `size`,
  `n_vocab`, architecture/modalities in current builds.
- `POST /completion` and OpenAI-compatible endpoints: `timings` with
  `prompt_n`, `prompt_ms`, `prompt_per_second`, `predicted_n`, `predicted_ms`,
  `predicted_per_second`, `cache_n`.
- `GET /metrics` (requires `--metrics`): Prometheus counters/gauges for prompt and
  generation tokens/throughput, requests, context high-water mark, and speculative
  decoding counters.
- `llama-bench -o json`: build commit/number, CPU/GPU info, backend list, model
  identity/size/parameters, all tested parameters, per-repetition samples,
  average and standard deviation.
- `llama-bench` explicitly does not include tokenization or sampling time.

### 4.5 CUDA/Pascal findings

- The GTX 1050 Ti Mobile is GP107M, compute capability 6.1, 4 GiB VRAM.
- The installed driver is 580.178.04; `nvidia-smi` reports CUDA runtime 13.0.
- CUDA 13.0 release notes state that Maxwell, Pascal, and Volta are
  feature-complete, and that offline compilation and library support for these
  architectures were removed in CUDA Toolkit 13.0. The 12.x series continues to
  support building for them.
- llama.cpp's CUDA CMake appends `50-virtual`, `61-virtual`, and `70-virtual`
  only when the toolkit version is less than 13. It also builds `75-virtual`,
  `80-virtual`, `86-real`, and newer architectures depending on toolkit version.
- Therefore the official CUDA 12.8 build is expected to include PTX for compute
  capability 6.1 and rely on driver JIT. A source build with the CUDA 13.4
  toolkit cannot target this GPU. This is a source-level conclusion; no prebuilt
  or local source build was executed here, and a CUDA 13.x-labeled prebuilt
  requires its own artifact-level inspection.
- A native build with `-DCMAKE_CUDA_ARCHITECTURES=61` is expected to work with
  CUDA 12.x and may avoid JIT and host/device architecture mismatches. It was not
  built because `nvcc`, `cmake`, and `ninja` are not installed, and installing
  them requires maintainer approval.
- The distro repository currently offers `cuda 13.4.3`, which is not suitable for
  a Pascal-targeted build. A CUDA 12.x toolkit would have to come from NVIDIA's
  archive or another explicitly approved source.
- `libcudart` and cuBLAS were not present in the local dynamic linker cache.
  Official CUDA Linux release archives ship a separate `cudart` asset; the
  prebuilt engine cannot be assumed to be self-contained.

Known upstream Pascal reports (not local tests):

- Open issue 30334: Pascal MMVQ fusion check reportedly misses compute
  capability 6.1 on P40.
- Open issue 30235: a Pascal DP4A MMQ configuration reportedly gives +12% prompt
  processing on sm_61 with a one-line tile change.
- Open issue 30524: PTX `61-virtual` code JIT-compiled on Turing has a
  host/device batch-limit mismatch; this is a different device, but it illustrates
  why PTX-only builds need targeted validation.
- Issue 25909 reports that official Windows CUDA prebuilts lacked sm_61 and that
  the reporter used Vulkan instead.

These reports are reasons to test, not proof about this laptop.

### 4.6 Vulkan findings

- The local Vulkan loader is 1.4.363. Two devices are visible: Intel UHD 630
  (Mesa 26.2.4) and NVIDIA GTX 1050 Ti (NVIDIA 580.178.04, Vulkan 1.4.312).
- Vulkan requires the system loader and GPU driver, not a CUDA toolkit.
- The desktop is rendered by the Intel iGPU, so the NVIDIA device is idle and
  available for compute. This is favorable for benchmarks, but it also means a
  Vulkan adapter that defaults to "first device" may select the Intel iGPU.
- Upstream has pre-Turing-specific Vulkan code paths (for example PR 29274
  changes NVIDIA `MUL_MAT_ID` row geometry except for pre-Turing devices). This
  indicates support, not a performance guarantee.
- No local Vulkan token-generation benchmark was run. CUDA-vs-Vulkan on Pascal is
  an open question.

## 5. PrismML fork details

### 5.1 Relationship to upstream

PrismML's repository is **not a separate inference architecture**. It is a fork
of llama.cpp/ggml:

- Repository: `PrismML-Eng/llama.cpp`, branch `prism`, MIT license.
- Its README says it tracks current mainline llama.cpp and adds the fork's low-bit
  formats and runtime features on top.
- At the access date the GitHub compare API reported 171 commits ahead of
  upstream master and 0 behind.
- Bonsai 2 `Q2_0` model support, including the Hadamard activation runtime, was
  merged upstream in PR #29600 (October 2026) and is available in upstream builds
  from `b11555` onward.
- `PQ2_0` is PrismML's own ggml type (group-128, id 142).
- `PTQ1_0` is under upstream review in PR #29672 and is fork-specific while open.
- MTP and DFlash2 drafters require fork binaries according to PrismML docs.

### 5.2 Additional capabilities

- Low-bit Bonsai 2 packings: `PQ2_0` (~2.13-2.20 bpw), `PTQ1_0`
  (~1.75-1.83 bpw), and upstream-compatible `Q2_0` (~2.28-2.33 bpw).
- Model-specific Hadamard transforms and sign flips; a runtime that knows the
  `Q2_0` type but not the transform can load a Bonsai 2 file and produce garbage.
- Speculative decoding paths: MTP and DFlash2, with checkpoint/runtime-specific
  compatibility.
- Backend work for CPU, CUDA, Metal, Vulkan, ROCm/HIP, and SYCL. The vendor's
  backend support table marks Vulkan `PQ2_0`/`PTQ1_0` as implemented but
  device/driver-dependent, and explicitly says it is source-level status, not a
  guarantee for every device.
- A commit dated 2026-10-19 adds a CUDA `PQ2_0` mat-vec path for 1-4 columns on
  Pascal (sm_61). This is direct evidence that the fork is working on this GPU
  class, but it is not a local benchmark.

### 5.3 Release and packaging

At access date, the latest PrismML release was `prism-b10787-73bfe6c`
(2026-10-19). Linux x64 assets include CPU, CUDA 12.4, CUDA 12.8, CUDA 13.3,
Vulkan, ROCm 7.2, and SYCL variants.

Reasons to support it later:

- The low-bit Bonsai models may fit a 4 GiB/8 GiB machine better than larger
  standard quantizations.
- Pascal-specific CUDA work and speculative decoding may provide practical value.
- It reuses the llama.cpp process/HTTP surface, so the adapter cost is moderate.

Reasons to defer it:

- It is a fast-moving fork with model-specific formats and kernels.
- Its value on 4 GiB Pascal is unproven.
- Vendor performance claims are not independent evidence.
- Supporting it expands the compatibility matrix and test burden.
- The MVP does not need Bonsai-specific formats to prove the control-plane value.

Recommendation: implement official llama.cpp first; add a PrismML build profile
only when a concrete model or capability need is demonstrated and tested.

## 6. Model format and inspection approach

GGUF is the initial model format. It is an extensible format with typed metadata
and tensors, specified by ggml. Stage 02 supports only little-endian GGUF v2/v3
and intentionally rejects big-endian and v1 files.

Implemented in Stage 02:

- Scan one explicitly selected directory for regular `*.gguf` files; there is no
  configured or default path. Non-recursive default, optional `--recursive`.
- Never follow symlinks; use non-following candidate opens on Unix. Non-UTF-8
  candidate paths and unreadable entries are reported as partial diagnostics.
- Read bounded headers, metadata, and tensor descriptors without loading or
  parsing tensor payloads. Optional `--hash` streams the complete file through
  SHA-256; without it, tensor bytes are not read.
- Extract GGUF version, architecture, model name, recognized file-type label and
  quantization hint, explicit parameter count if present, context length and common
  block/attention/embedding fields, tokenizer model and array summaries, tensor
  types, alignment, file size, modification time, and optional SHA-256.
- Emit no token vocabulary, scores, merge pairs, added-token values, chat-template
  text, Hugging Face tokenizer JSON, prompts, or tensor data.
- Treat model files as untrusted input with checked lengths, explicit limits,
  typed errors, no panics on malformed fixtures, and no writes.
- Provide conservative filename hints for `mmproj`, `mtp`, LoRA, vocabulary, and
  split-shard suffixes. Hints never assert identity.

Not implemented or verified:

- shard grouping or reconstruction; each shard and sidecar is a separate record;
- a cache, database, `models list`, or watch mode;
- real-model parsing or hashing (no real model was available);
- model loading, engine compatibility, or inference;
- post-load metadata cross-check against `/v1/models` and `/props`.

The last item remains required before engine-launch decisions rely on catalog
metadata. A mismatch must remain a diagnostic signal, not be silently resolved.

Parser choice: Stage 02 selected a minimal in-tree metadata reader rather than an
external parser. The reader is narrow, bounded, and auditable for this catalog
subset. Crates such as `gguf` and `gguf-rs` were not audited, and `gguf-py`
`gguf_dump.py` remains a development reference only. The selected parser is not a
general-purpose loader or a full GGUF validator; see
[GGUF_CATALOG.md](GGUF_CATALOG.md).

## 7. Measurement approach by engine

| Measurement | Official llama.cpp | PrismML fork | Notes |
| --- | --- | --- | --- |
| Build identity | `--version`, `/props` `build_info`, binary SHA-256 | Same; fork tag/commit | Record release tag or source commit when available; keep the install path separate. |
| Device visibility | `--list-devices`, startup logs | Same | Vulkan exposes Intel and NVIDIA. |
| Prompt throughput | `llama-bench -p ... -n 0 -o json`; server `prompt_per_second` | Same | Keep prompt and generation separate. |
| Generation throughput | `llama-bench -p 0 -n ... -o json`; server `predicted_per_second` | Same | `llama-bench` excludes sampling/tokenization. |
| TTFT | Client-side streaming request; server `prompt_ms` as a cross-check | Same | Queue/cache effects must be controlled. |
| Peak VRAM | NVML or `nvidia-smi` sampling; device-level free/used | Same | Per-process VRAM may be unavailable on some drivers. |
| Peak RAM | `/proc/<pid>/smaps_rollup`, `VmHWM`; host `MemAvailable` | Same | Child processes complicate accounting. |
| Swap/pressure | `/proc/pressure/memory`, `VmSwap`, zram stats | Same | zram is not disk swap; report it separately. |
| Model load time | Process start to `/health` 200; logs | Same | Distinguish cold page cache from warm. |
| Failures | Exit code/signal, logs, OOM evidence | Same | Record, do not discard. |
| Speculation | n/a | `timings` draft/accepted fields; vendor docs | Build/model-specific. |

## 8. Open questions that block support claims

1. Does the official CUDA 12.8 Linux binary actually initialize on cc 6.1 with
   the installed driver, and does it use PTX JIT or a real 6.1 binary?
2. Is a native CUDA 12.8 `-DCMAKE_CUDA_ARCHITECTURES=61` build materially faster
   or more stable than the prebuilt `61-virtual` path?
3. Does Vulkan select the NVIDIA GPU by default, and what is the stable device
   selector on this machine?
4. Is Vulkan faster, slower, or equivalent to CUDA 12.8 for prompt processing and
   generation on this 4 GiB Pascal GPU?
5. Which model sizes and context lengths remain usable under the RAM/VRAM budget?
6. Does PrismML's fork provide a measurable benefit on this laptop, and do its
   Pascal PQ2_0 and speculative paths work outside the vendor's tested hardware?
7. What real-model and engine cross-checks are needed before the Stage 02 catalog
   is trusted for launch decisions? The narrow parser choice is made; external
   parser audits and real-model validation remain open.
8. How should `--fit`-resolved values be captured for reproducible comparisons?
9. What defines "usable" rather than merely "successfully loaded" for this
   project's users?
10. Which engine build should be pinned as the first supported baseline? Deferred
    to Stage 04 integration and its build/checksum/runtime/launch checks.

## 9. Sources

Engine sources were accessed 2026-10-20. GGUF format sources were rechecked
2026-10-21; see the dated research notes for per-claim confidence and additional
links.

- llama.cpp repository and README: <https://github.com/ggml-org/llama.cpp>
- llama.cpp build guide: <https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md>
- llama.cpp server API: <https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md>
- llama.cpp benchmark tool: <https://github.com/ggml-org/llama.cpp/blob/master/tools/llama-bench/README.md>
- llama.cpp release process: <https://github.com/ggml-org/llama.cpp/blob/master/docs/release.md>
- llama.cpp CUDA architecture logic: <https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-cuda/CMakeLists.txt>
- llama.cpp release workflow: <https://github.com/ggml-org/llama.cpp/blob/master/.github/workflows/release.yml>
- llama.cpp nightly b11756: <https://github.com/ggml-org/llama.cpp/releases/tag/b11756>
- llama.cpp v0.7.0 tag: <https://github.com/ggml-org/llama.cpp/releases/tag/v0.7.0>
- GGUF specification: <https://github.com/ggml-org/ggml/blob/master/docs/gguf.md>
- GGUF header: <https://github.com/ggml-org/llama.cpp/blob/master/ggml/include/gguf.h>
- GGUF loader/writer: <https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/gguf.cpp>
- GGML tensor types: <https://github.com/ggml-org/llama.cpp/blob/master/ggml/include/ggml.h>
- GGUF Python constants: <https://github.com/ggml-org/llama.cpp/blob/master/gguf-py/gguf/constants.py>
- NVIDIA CUDA 13.0 release notes: <https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html>
- NVIDIA CUDA 12.8 release notes: <https://docs.nvidia.com/cuda/archive/12.8.0/cuda-toolkit-release-notes/index.html>
- NVIDIA CUDA GPU compute capability table: <https://developer.nvidia.com/cuda-gpus>
- PrismML fork: <https://github.com/PrismML-Eng/llama.cpp>
- PrismML llama.cpp documentation: <https://docs.prismml.com/run/llamacpp>
- PrismML formats and runtime support: <https://docs.prismml.com/download/formats>
- PrismML backend support: <https://github.com/PrismML-Eng/Bonsai-demo/blob/main/BACKEND-SUPPORT.md>
- PrismML release: <https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10787-73bfe6c>
- Upstream Bonsai 2 merge PR: <https://github.com/ggml-org/llama.cpp/pull/29600>
- Upstream PTQ1_0 PR: <https://github.com/ggml-org/llama.cpp/pull/29672>
- PrismML Pascal PQ2_0 commit: <https://github.com/PrismML-Eng/llama.cpp/commit/483bdaba>
- Pascal/prebuilt reports: [25909](https://github.com/ggml-org/llama.cpp/issues/25909),
  [29174](https://github.com/ggml-org/llama.cpp/issues/29174),
  [30235](https://github.com/ggml-org/llama.cpp/issues/30235),
  [30334](https://github.com/ggml-org/llama.cpp/issues/30334),
  [30524](https://github.com/ggml-org/llama.cpp/issues/30524)
- Vulkan pre-Turing path: <https://github.com/ggml-org/llama.cpp/pull/29274>
- Iced 0.14.0 (future UI): <https://github.com/iced-rs/iced/releases/tag/0.14.0>
