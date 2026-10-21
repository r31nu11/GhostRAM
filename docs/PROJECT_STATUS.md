# Project status

**Snapshot date:** 2026-10-21.
**Current stage:** Stage 03 — Memory planner and safety budgets.
**Stage state:** Implementation and documentation complete; **pending formal
acceptance**.
**Branch:** `dev`.
**Commit state:** Stage 03 content commit
`2dda210a28f88bee4dc6c2538db9224ca1cdb5d6`; the final report/status commit is its
descendant and is identified with `git log -1` after hand-off. Both commits use the
local maintainer identity `r31nu11 <r31nu11@mail.ru>`.
**Publication:** the maintainer authorized publication of the Stage 00/01/02/03
development history to `origin/dev`. This is not authorization for a tag, GitHub
Release, artifact, or promotion to `main`.
**Working tree:** clean after the final report/status commit; any remaining path is
explained in the Stage 03 report.

## Summary

Stage 00 was accepted on 2026-10-20 and ADR-0001 through ADR-0005 were adopted at
the same time. Stage 01 was accepted on 2026-10-21. Stage 02 was accepted on
2026-10-21. Stage 03 adds the fifth workspace crate and a pure, read-only memory
planning path:

- `ghostram-planner` is a pure calculation core. It accepts compact model metadata,
  a resource snapshot, and validated options; it does not scan, probe, read files,
  start subprocesses, call an engine, or persist state.
- `ghostram models plan <path>` requires one explicitly selected regular GGUF file.
  It does not scan a directory, hash automatically, use a default model path, or
  write a plan. It collects only host memory and NVIDIA memory observations.
- Every planner quantity is evidence-tagged as `observed`, `explicit_input`,
  `derived`, `estimated`, `product_default`, or `unknown`. Unknown quantities carry
  a reason and no value; missing data is never zero.
- Modeled weights are only a low-confidence GGUF file-size proxy. The plan does not
  infer exact tensor allocation, quantization overhead, mmap residency, or GPU layer
  counts.
- KV cache is limited to the documented `llama_uniform_full_attention_v1` profile
  and `uniform_kv_cache_v1` formula. F16 is the visible default; F32 is supported.
  Special attention layouts, missing dimensions, unsupported architectures, and
  quantized KV produce explicit unavailability rather than invented numbers.
- Budgets use checked `u64` arithmetic and non-negative effective values. The
  product defaults are 1,536 MiB host reserve and 1,024 MiB device reserve.
- Exactly two scenarios are evaluated: CPU-resident weights plus KV, and a
  hypothetical full-device weight placement with host KV. Outcomes are
  `estimated_components_within_budget`,
  `estimated_components_exceed_budget`, or `insufficient_data`.
- `ghostram.models.plan/1` is a new versioned JSON contract.
  `ghostram.doctor/1` and `ghostram.models.scan/1` are unchanged.
- Deterministic unit and CLI integration tests cover arithmetic, reserves,
  boundaries, overflow, formula/profile behavior, missing metadata, cache types,
  proxy labeling, device uncertainty, JSON round-trips, malformed files, output
  purity, and exit codes.

**No real GGUF model was available or tested.** All parser and planner evidence is
from deterministic synthetic fixtures and synthetic resource snapshots. No engine is
installed, so no model allocation, launch, inference, or benchmark was observed. No
inference, GPU compute, model download, network test, or hardware measurement was
performed in Stage 03.

## Current capabilities

| Capability | State |
| --- | --- |
| `ghostram doctor` / `doctor --json` | Implemented read-only diagnostics; source-only |
| Linux hardware, GPU, Vulkan, tool, and engine probes | Implemented read-only; no engine installed |
| GGUF v2/v3 little-endian metadata parser | Implemented with explicit limits; synthetic fixtures only |
| `ghostram models scan` / `--json` | Implemented; one user-selected directory; source-only |
| Recursive scan | Implemented as opt-in `--recursive`; depth and entry limits apply |
| SHA-256 | Optional explicit `--hash`; streaming and complete-read required |
| `ghostram models plan` / `--json` | Implemented; one explicit file; source-only; `ghostram.models.plan/1` |
| Weight estimate | File-size proxy only; low confidence; shard/malformed cases unavailable |
| KV-cache estimate | Narrow uniform full-attention `llama` profile; F16 default/F32; synthetic only |
| Budget/scenario engine | Checked integer arithmetic, configurable reserves, two qualified scenarios |
| Plan persistence | **Not implemented**; plans are ephemeral |
| Stable model identity, shard grouping, `models list`, cache, database | **Not implemented**; earlier scope superseded |
| Real-model/engine cross-check | **Not implemented**; no model or engine available |
| Model loading, launch, inference, runtime governor | **Not implemented** |
| Candidate generation, tuner, store, GUI, benchmarks | **Not implemented** |

## Stage 03 decisions

- No new ADR was required. Adding `ghostram-planner` follows the crate-split rule in
  ADR-0002 and does not rewrite an accepted decision. The planner is a fifth crate
  because its pure contract has a distinct test seam and multiple future adapters.
- The planner core is pure. The CLI owns file inspection and memory/NVIDIA
  observation; the planner never performs I/O, subprocess work, or engine calls.
- The plan schema is `ghostram.models.plan/1`. Existing scan and doctor schemas were
  not changed.
- `EvidenceClass` includes `product_default` so policy defaults are visibly
  different from user inputs, observations, and estimates.
- The only weight method is `gguf_file_size_proxy`. A shard filename, non-parsed file,
  or missing filesystem size leaves total weights unknown.
- The only KV profile is `llama_uniform_full_attention_v1`; the formula identifier is
  `uniform_kv_cache_v1`; the method serializes as `uniform_full_attention`.
- Recognized layout-marker presence is tracked in non-serialized parser fields. The
  marker makes KV unavailable conservatively even if its value appears disabled.
  `ghostram.models.scan/1` JSON remains unchanged.
- The `llama` defaults are evidence-tagged: missing `attention.head_count_kv`
  defaults to the query head count, and missing key/value widths default to
  `embedding_length / head_count`.
- F16 and F32 are the only modeled KV element types. Quantized caches are deferred
  because block layout, scale metadata, backend alignment, and kernel behavior are
  not a simple linear byte multiplier.
- Budgets subtract a configured reserve only when inputs are known. An
  availability at or below the reserve yields a non-negative effective zero marked
  unusable, never a negative value.
- The device scenario is hypothetical full-device weight placement with host KV. It
  does not model partial offload or GPU-resident KV.
- The context estimate is a host-budget upper bound after the file-size proxy. It is
  capped at the planner's option limit and warns when it exceeds declared context;
  it is not an engine context limit.
- The plan never emits an unqualified `model_fits`, `launchable`, `supported`, or
  `safe_to_run`. Launch refusal is deferred because Stage 03 has no launch path.

## Validation snapshot

Exact commands and output for the committed state are recorded in the
[Stage 03 report](../reports/stages/stage-03-memory-planner.md). The final
hand-off reruns the suite over the completed history.

| Check | Observed result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed (no output) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | Passed; exact count in the Stage 03 report |
| `cargo build --workspace --locked` | Passed |
| `python3 scripts/check_docs.py` | Passed; exact counts in the Stage 03 report |
| `bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD` | Passed at the Stage 03 content commit; rerun over the final HEAD during hand-off and reported outside the self-referential commit |
| Local author identity | `r31nu11` / `r31nu11@mail.ru` |
| Complete diff inspection | Performed before staging |

## Environment snapshot

Stage 03 did not take a new hardware test measurement. The last accepted read-only
hardware snapshot is in the
[Stage 01 report](../reports/stages/stage-01-minimal-rust-workspace.md): the primary
laptop has a 4 GiB Pascal GPU and 7.4 GiB RAM; the secondary machine has not been
accessed. CLI integration tests invoked the lightweight memory/NVIDIA path with a
restricted `PATH`; those runs are test plumbing, not a hardware validation record.
No engine was installed. No real model was supplied, downloaded, scanned, or
planned, and no benchmark was run.

## Not done and deferred

- No real GGUF file, engine cross-check, or memory-allocation measurement. Synthetic
  fixtures cannot prove the file-size proxy or KV formula matches a real engine.
- No backend workspace, graph allocation, driver overhead, mmap, alignment, or
  quantization-overhead estimate; runtime overhead remains explicitly unknown.
- No real-model validation of architecture defaults, special layouts, or tokenizer
  behavior. KV support is deliberately narrow.
- No quantized KV cache, latent attention, sliding-window, recurrent, per-layer
  array, multi-architecture, or shard-grouped planning.
- No candidate-list generation, optimizer, Pareto search, or tuner.
- No launch refusal, override acknowledgment, runtime pressure sampling, process
  supervision, or engine adapter.
- No plan persistence, history, cache, database, or stable model ID.
- No GPU layer-count inference, partial offload, or multi-GPU planning.
- No Vulkan/AMD/Intel device-memory budget; planning devices are NVIDIA only.
- No release artifact, installer, tag, GitHub Release, or `main` promotion.
- No Stage 04 work. Stage 04 remains blocked until Stage 03 is accepted.

## Next stage

Stage 04 (engine adapter and supervised runtime) is next but blocked on formal
Stage 03 acceptance. Its planning already names the CPU baseline, official Linux
CUDA 12.8 Pascal candidate, official Vulkan candidate, and a source build targeting
`sm_61`, but none may be installed or run without separate maintainer approval.
The maintainer must review the Stage 03 report, all deferred scope, and the
validation evidence before authorizing Stage 04.
