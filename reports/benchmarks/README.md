# Benchmark reports

**Status:** No benchmark has been executed. This directory intentionally contains
no result files.

This directory holds curated benchmark reports. Raw run data belongs with the run
record; a report is a reviewed interpretation of that data, not a replacement for
it.

## 1. Rules

1. Every number is labeled `estimated`, `measured`, or `unknown`.
2. A measured number links to a run manifest with hardware, engine build, model
   hash, resolved configuration, workload, raw samples, and failures.
3. No fabricated, copied, or "representative" numbers. If a run was not performed,
   write "not measured".
4. Report failures, exclusions, variance, and missing fields. Do not hide runs that
   did not fit the narrative.
5. Do not compare runs with different models, builds, backends, contexts, power
   states, or workloads without labeling the difference and justifying it.
6. Do not claim output quality or correctness from a throughput benchmark.
7. Do not publish a report that omits the engine build or device.
8. Keep raw logs and samples available; a summary that cannot be traced to them is
   not acceptable.

## 2. Naming

Use a stable, sortable name:

```text
YYYY-MM-DD-<machine>-<engine>-<backend>-<model>-<workload>.md
YYYY-MM-DD-<machine>-<engine>-<backend>-<model>-<workload>.json
```

Example (illustrative, not an existing report):
`2026-11-02-primary-llamacpp-cuda128-qwen3-0.6b-pp-tg.md`.

A report may cover a small matrix; use a matrix name instead of repeating every
model in the filename. Record the exact engine build and model hash inside.

## 3. Required report sections

1. **Question and decision rule** — what was being tested and what result would
   change a decision.
2. **Environment** — machine, OS/kernel, CPU, RAM, swap, GPU, VRAM, driver,
   Vulkan/CUDA runtime, power state, governor.
3. **Engine, installation, and model identity** — executable hash, version/build
   info, release tag or commit when available, installation path and discovery
   method, model path/hash, quantization, and context limit.
4. **Configuration** — exact resolved arguments, including defaults filled by the
   adapter and values adjusted by the engine. State whether `--fit` was disabled.
5. **Workload** — prompt source/hash, token counts, generation length, sampling,
   concurrency, warmup, repetitions.
6. **Raw data location** — path or record ID for samples and logs.
7. **Results** — prompt processing, generation, TTFT, load time, peak RAM/VRAM,
   swap/pressure, separated by metric and unit.
8. **Statistics** — sample count, median, spread, exclusions, and pre-declared
   outlier handling.
9. **Failures and missing data** — OOM, timeouts, crashes, unknown fields.
10. **Interpretation** — limited to what the evidence supports.
11. **Reproduction** — exact commands or procedure, with secrets removed.
12. **Review** — reviewer, date, and whether the report meets this contract.

## 4. Data and privacy

- Do not commit model weights, engine binaries, API keys, or private prompts.
- Store prompt hashes or bounded, non-sensitive prompts.
- Strip usernames and hostnames from raw logs where they are not needed.
- Large raw logs may live outside Git with a recorded hash and location, but the
  report must say where they are.

## 5. Current state

| Item | State |
| --- | --- |
| Reports | None. |
| Runs | None. |
| Raw samples | None. |
| Hardware validation | None. |
| Engine installed | No. |
| Model available | No. |

See [BENCHMARKING.md](../../docs/BENCHMARKING.md) for methodology and
[Stage 00](../stages/stage-00-technical-reconnaissance.md) for the environment
baseline.
