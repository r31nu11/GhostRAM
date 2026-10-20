# Benchmarking methodology

**Status:** Stage 00 methodology. **No benchmark has been executed by GhostRAM.**
There are no benchmark results in this repository, and no number in this document
is a measurement.

This document defines how future experiments must be designed, recorded, and
reported. It is intentionally stricter than a typical "run llama-bench" guide
because the project's value depends on trustworthy evidence.

## 1. Questions and claim levels

Every experiment must start with a question and a decision rule. Examples:

- Does Vulkan or CUDA 12.8 give higher generation throughput on the GTX 1050 Ti
  at the same model, context, and offload?
- How much does context length reduce throughput before the run becomes unusable?
- Does a KV-cache quantization change speed or memory enough to justify its risk?
- Does PrismML's low-bit format make a model usable that otherwise does not fit?

Claims have three levels:

| Level | Meaning | Allowed wording |
| --- | --- | --- |
| `estimated` | Derived from metadata, formulas, or prior data. | "Estimated", "predicted", "expected". |
| `measured` | Observed in a recorded run with a complete manifest. | "Measured on <machine> with <build>". |
| `unknown` | No adequate evidence. | "Not measured", "unknown". |

A measured value without a run manifest is invalid. An estimate never becomes a
measurement because it was repeated or printed in a report.

## 2. Provenance manifest

Every run record must include:

- GhostRAM version and schema version.
- UTC start/end timestamps and run ID.
- Hardware fingerprint: OS, kernel, CPU model, physical/logical cores, RAM,
  swap/zram, GPU model, VRAM, driver, Vulkan/OpenCL devices, storage path.
- Power state (AC/battery), CPU governor, and any thermal or clock settings that
  were observed.
- Engine build identity: family, binary SHA-256, version/build info, and release
  tag or source commit when available.
- Installation record: executable path and discovery method.
- Runtime context and capability observations: backend, selected device, driver
  or runtime version, visible devices, and dependency evidence. Include the exact
  device selector used when multiple Vulkan devices exist.
- Model identity: path, file size, SHA-256, architecture, parameter count,
  quantization, context limit, and shard/draft files.
- Exact resolved configuration: context, batch/ubatch, GPU layers, KV cache type,
  Flash Attention, fit settings, threads, parallel slots, and any engine-specific
  flags. Record whether each value was user-set, defaulted, adapter-filled, or
  engine-resolved.
- Workload: prompt text or hash, prompt length, generation length, sampling
  settings, warmup, repetitions, and concurrency.
- Raw samples, summary statistics, failures, retries, and exclusions.
- The command line as an argument array, never as a shell string with secrets.

If a field is unavailable, record it as `unknown` with the reason. Do not omit it
silently.

## 3. What to measure

### 3.1 Prompt processing

Prompt-processing throughput (`prompt_per_second` or `pp`) measures how fast the
engine ingests input tokens. Report it separately from generation. Do not average
the two into a single "tokens per second" number.

### 3.2 Generation

Generation throughput (`predicted_per_second` or `tg`) measures output tokens per
second after the prompt is processed. Report the token count and the measurement
window.

### 3.3 Time to first token (TTFT)

TTFT is measured client-side from request send to the first streamed content
token. It includes queueing, prompt processing, and scheduling. Record the
endpoint, prompt length, cache state, and whether other requests were active. A
server-side `prompt_ms` value is a cross-check, not a replacement for the
client-observed value.

### 3.4 Memory and pressure

- Peak process RAM: `/proc/<pid>/status` `VmHWM` and `smaps_rollup` where
  available.
- Host available memory: `MemAvailable`, not `MemFree`.
- Swap: `VmSwap`, `swapon --show`, and zram statistics. zram is compressed RAM,
  not disk swap; report it separately.
- Pressure: `/proc/pressure/memory` some/full averages.
- GPU memory: `nvidia-smi` or NVML sampling, with the sampling interval recorded.
  Per-process VRAM may be unavailable; do not invent it.
- Load time: from process spawn to a successful health check, with cold/warm page
  cache distinguished.

### 3.5 Failures

Record startup failures, OOM kills, driver resets, timeouts, malformed responses,
and user cancellations. A failed run is data. Do not delete it because it is
inconvenient.

## 4. Controls

Benchmarks on a laptop are noisy. At minimum:

1. Run on AC power and record the power state. Do not compare battery and AC runs
   as if they were equivalent.
2. Record the CPU governor and do not change it automatically. If a fixed
   governor is approved for an experiment, record the change and restore it.
3. Close other heavy workloads and record obvious background activity.
4. Run one inference engine at a time.
5. Warm up the model and page cache, then state whether reported runs are cold or
   warm.
6. Set all relevant engine arguments explicitly. `llama-server` enables `--fit`
   by default; for controlled comparisons either disable fit or record the
   engine-resolved values. A silent fit adjustment invalidates a comparison.
7. Select the intended device explicitly. This laptop exposes both Intel UHD 630
   and NVIDIA GTX 1050 Ti through Vulkan; "first device" is not a safe default.
8. Keep the model, quantization, context, batch, KV type, and offload identical
   when comparing backends.
9. Allow cooldown between sustained runs and record thermal observations if
   available.
10. Use the same prompt and generation lengths for every candidate in a
    comparison.

## 5. Repeats and statistics

- Use a warmup run that is not included in the summary.
- Use at least five measured repetitions for screening comparisons, and more for
  final claims. This is an initial proposal and must be calibrated.
- Report every sample, the median, the standard deviation or interquartile range,
  and the number of valid samples.
- Do not report only the best run.
- Pre-declare outlier handling. If a sample is excluded, record why.
- Treat differences smaller than run-to-run variance as inconclusive.
- Do not pool runs from different models, builds, backends, or power states
  without labeling the grouping and testing whether pooling is justified.

## 6. Workloads

Initial workload classes, to be fixed before execution:

| Workload | Purpose | Proposed shape |
| --- | --- | --- |
| `pp` | Prompt-processing throughput | Prompt of fixed token length, zero or minimal generation. |
| `tg` | Generation throughput | Minimal prompt, fixed generation length. |
| `mixed` | Realistic chat | Fixed multi-turn prompt and fixed output cap. |
| `ttft` | Responsiveness | Streaming request; record first-token latency. |
| `context` | Context scaling | Same workload at several context sizes. |

Token counts must come from the engine's tokenizer or a recorded tokenization
method, not from a character-count approximation.

`llama-bench` measures prompt processing and generation but explicitly excludes
tokenization and sampling time. Server timings include different boundaries.
Never compare the two without labeling the difference.

## 7. Experiment sketches (not executed)

These are plans for Stage 05, not results.

### E1 — Baseline characterization

- CPU-only and one GPU backend, one small model, fixed context.
- Establish variance, warmup behavior, and a usable repeat count.
- Record idle memory, peak memory, swap, and pressure.

### E2 — CUDA 12.8 vs Vulkan

- Same model, quantization, context, batch, KV type, and offload.
- Explicit NVIDIA device selection for Vulkan.
- Compare prompt processing and generation separately.
- Record whether CUDA uses native SASS or PTX JIT.

### E3 — Offload and context sweep

- Vary GPU layers and context within safe budgets.
- Identify the boundary where memory pressure or throughput collapse occurs.
- Do not present a fit that merely loads as usable.

### E4 — KV-cache precision

- Compare supported KV types at a fixed context and batch.
- Measure both throughput and memory; record model-quality caveats separately.

### E5 — PrismML optional profile

- Only after the upstream adapter is stable and a suitable model is available.
- Test model load, output sanity, memory, and throughput; do not assume vendor
  claims transfer to this GPU.

Each experiment needs a pre-declared hypothesis, candidate list, stopping rule,
and safety budget before it runs.

## 8. Reporting format

A benchmark report contains:

1. Question and decision rule.
2. Environment and provenance manifest.
3. Engine and model identity.
4. Exact configurations, including resolved/defaulted values.
5. Workload definition.
6. Raw samples or a link to them.
7. Summary statistics with units and variance.
8. Failures, exclusions, and missing data.
9. Interpretation limited to what the data supports.
10. Reproduction instructions.

Reports live under `reports/benchmarks/` and follow
[the benchmark report contract](../reports/benchmarks/README.md). JSON records are
the source of truth; Markdown is generated or checked against them. A hand-edited
summary that disagrees with the JSON is a defect.

## 9. Validity checklist

Before publishing a result, confirm:

- [ ] The question and decision rule were declared before the run.
- [ ] The manifest is complete; unknown fields have reasons.
- [ ] The engine build and model hash are recorded.
- [ ] The selected device is the intended one.
- [ ] `--fit` and other engine-resolved values are recorded or disabled.
- [ ] Prompt and generation metrics are separate.
- [ ] Raw samples and failures are retained.
- [ ] Repeats and variance are reported.
- [ ] No number is labeled measured without a run record.
- [ ] The conclusion does not exceed the evidence.

## 10. Prohibited practices

- Fabricating, estimating, or copying benchmark numbers into a report as if they
  were observed.
- Reporting a single best run as the result.
- Hiding failed configurations or OOM events.
- Comparing runs with different models, builds, contexts, or power states without
  disclosure.
- Claiming a speedup from a change that was not isolated.
- Using benchmark results as proof of correctness or output quality.
- Publishing a report that omits the hardware and engine identity.
