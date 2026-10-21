# Memory planner behavior

**Status:** Stage 03 implementation complete and pending formal acceptance.
Source-only; there is no release artifact or installer.
**Implementation:** `crates/ghostram-planner`, with the `models plan` command in
`crates/ghostram-cli`.
**JSON contract:** [PLAN_SCHEMA.md](PLAN_SCHEMA.md).
**Formula evidence:** [KV-cache planning research, 2026-10-21](research/2026-10-21-kv-cache-planning.md).

The planner compares a small, explicitly enumerated set of modeled components with
host and device budgets. It is deliberately narrower than a model loader or an
inference launcher. A plan is an evidence-labelled estimate; it is not a claim
that a model fits, loads, launches, or runs safely.

## 1. Purpose and non-goals

The planner answers a limited question:

> Given one explicitly selected GGUF file, documented planning options, and
> current host/NVIDIA observations, what do the modeled weights and KV cache
> imply for two placement scenarios?

It does **not**:

- scan a directory, choose a model automatically, or read any file other than the
  selected file;
- hash the model automatically or require a content hash;
- parse tensor payloads or validate tensor allocation;
- infer a stable model identity, group shards, combine adapters, or reconstruct a
  multi-file model;
- model exact quantization overhead, alignment, mmap behavior, backend workspace,
  graph allocations, compute buffers, or driver overhead;
- predict GPU layer counts or partial offload behavior;
- install, start, query, or supervise an engine;
- run inference or a benchmark;
- persist plans, create a cache or database, or contact a network service;
- refuse or authorize a launch. There is no runtime to govern in Stage 03.

## 2. Boundary and data flow

The calculation core is pure. `ghostram_planner::build_plan` accepts:

1. a compact `ModelInput` derived from one `FileCatalogEntry`;
2. a `ResourceSnapshot`; and
3. validated `PlanOptions`.

The core performs no filesystem access, subprocess execution, probing, logging, or
engine calls. It does not read environment variables or global configuration.

The CLI performs the explicitly visible boundary work:

```text
explicit file path
  -> ghostram_models::inspect_file
       - absolute path, symlink and regular-file checks
       - bounded GGUF metadata parse; no hash and no tensor payload read
  -> collect_planning_observations
       - /proc/meminfo host memory
       - nvidia-smi NVIDIA device memory
  -> ghostram_planner::build_plan
       - pure checked arithmetic and evidence classification
  -> human output or one ghostram.models.plan/1 JSON envelope
```

`collect_planning_observations` deliberately runs only the memory and NVIDIA
collectors. It does not run storage, tools, engine, Vulkan, or other doctor probes.
The plan therefore has no Vulkan device budget and no engine capability claim.

The selected path is made absolute without canonicalizing it. A missing path,
directory, non-regular file, symbolic link, or inaccessible path is a command
error with exit code 1. `inspect_file` also rejects a resolved path that is not
valid UTF-8. A readable file with malformed or unsupported GGUF content is a
successful plan with unavailable components; the file-size observation and the
parse failure remain visible.

## 3. Command and options

```text
ghostram models plan <path> [options] [--json]
```

The path is required. There is no default directory, model search, or implicit
selection. A path beginning with `-` can be supplied after the `--` terminator.

| Option | Meaning | Default | Validation |
| --- | --- | --- | --- |
| `--context <tokens>` | Context per sequence | 4,096 | 1 to 1,048,576 inclusive |
| `--sequences <n>` | Independent full-context sequences | 1 | 1 to 256 inclusive |
| `--kv-cache <type>` | KV element type, `f16` or `f32` | `f16` | Case-insensitive; no quantized values |
| `--ram-reserve-mib <mib>` | Host reserve withheld from available RAM | 1,536 MiB | Non-negative `u64`; MiB must fit in `u64` bytes |
| `--vram-reserve-mib <mib>` | Device reserve withheld from free VRAM | 1,024 MiB | Non-negative `u64`; MiB must fit in `u64` bytes |
| `--device <index>` | Explicit NVIDIA device index | First observed device | Non-negative `u32` |
| `--json` | Print one versioned JSON envelope only | Human output | May appear once |
| `-h`, `--help` | Print command help | — | — |

Each value option may appear at most once. Unknown options and a second positional
path are usage errors. Invalid values are rejected before file inspection. The
defaults are product-policy defaults, not measurements or universal
recommendations.

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | The selected file was inspected and a plan was produced, including a plan with unavailable components or insufficient data. |
| `1` | The selected path could not be selected safely, or output failed. Path failures happen before any plan output starts; an output failure can leave partial stdout. |
| `2` | The invocation was invalid. Nothing is written to stdout on this error. |

The command does not write files, create a cache, hash the model, or modify the
selected file.

## 4. Evidence classes

Every planner quantity is wrapped in an evidence-tagged object. A value is never
shown without its class.

| Class | Meaning | Example |
| --- | --- | --- |
| `observed` | A current file or system observation. | File size, `MemAvailable`, `nvidia-smi` total/used memory. |
| `explicit_input` | A value supplied by the user or caller. | `--context 8192`, an explicit device index. |
| `derived` | Exact checked arithmetic from known values. | Effective budget, headroom, shortfall, free VRAM from total minus used. |
| `estimated` | A proxy, formula, or explicit planning assumption. | File-size weight proxy, uniform KV formula, context upper bound. |
| `product_default` | A documented policy default chosen by this build. | 4,096 context, F16 cache, 1,536 MiB host reserve, 1,024 MiB device reserve. |
| `unknown` | No usable value exists. The object has a reason and no value field. | Missing `MemAvailable`, unsupported KV layout, unmodeled runtime overhead. |

Rules enforced by the implementation and tests:

- Missing data never becomes zero.
- An estimate is never labeled as a measurement.
- A product default is never labeled as an observation or a user input.
- An explicit option remains visible as `explicit_input`; the planner does not
  silently replace it with a default.
- `Unknown` variants have no `value` field, so a consumer cannot mistake a missing
  quantity for a numeric zero.
- The `derived` class is used only when all inputs are known and arithmetic is
  checked.

## 5. Model-weight proxy

The only weight estimate is:

```text
model_weights_estimate = observed GGUF file size
method                 = gguf_file_size_proxy
evidence               = estimated
```

The file size is observed before parsing and is retained as the `basis`. The
estimate is produced only when the file parsed as a supported GGUF structure and
the filename does not identify a shard of a multi-file model.

The proxy is low-confidence. It does **not** model:

- tensor offsets, alignment padding, or unused regions;
- quantization block layout or per-tensor storage overhead;
- mmap or page-cache residency;
- a GPU layer split or partial offload;
- a tokenizer, multimodal projector, adapter, or other sidecar that is not part of
  the selected file;
- actual resident memory after an engine loads the model.

A malformed, unsupported, unreadable, or changed file leaves the weight estimate
unknown. A filename that looks like shard `n` of `total > 1` also leaves total
weights unknown because Stage 03 does not group or reconstruct shards. Sidecar
filename hints are warnings only; the planner never combines a sidecar with a base
model.

## 6. KV-cache profile and formula

Stage 03 supports one profile:

```text
profile:      llama_uniform_full_attention_v1
architecture: llama
method:       uniform_full_attention
formula:      uniform_kv_cache_v1
```

The accepted dimensions are:

| GGUF key | Use | Default when absent |
| --- | --- | --- |
| `llama.block_count` | Number of transformer layers | None; unavailable |
| `llama.embedding_length` | Used to derive default head widths | None; unavailable |
| `llama.attention.head_count` | Query heads | None; unavailable |
| `llama.attention.head_count_kv` | KV heads | Query head count, labeled `default` |
| `llama.attention.key_length` | Key width | `embedding_length / head_count`, labeled `default` |
| `llama.attention.value_length` | Value width | `embedding_length / head_count`, labeled `default` |

The formula is:

```text
elements = context_tokens * layers * kv_heads
         * (key_dimension + value_dimension) * sequences
bytes    = elements * bytes_per_element
```

`F16` uses 2 bytes per element and `F32` uses 4. All multiplications and additions
are checked `u64` operations. The plan records the context, sequence count,
layers, query heads, KV heads, key/value widths, cache type, and total elements
when available. Human output also displays the derived per-token bytes.

The estimate assumes uniform full attention on every modeled layer. It does not
model:

- sliding windows or mixed sliding/full-attention layer patterns;
- separate sliding-window key/value widths;
- latent attention, recurrent state, or non-transformer cache structures;
- per-layer arrays or non-uniform head widths;
- backend padding, alignment, or workspace;
- unified versus independent sequence-cache sharing beyond the explicit
  independent-full-context assumption;
- quantized KV storage. Only F16 and F32 are accepted.

The parser records recognized layout-key presence outside the
`ghostram.models.scan/1` JSON. If a recognized marker is present for the selected
architecture, KV is unavailable even when the marker's value might appear
disabled. This is conservative: the planner does not guess that a special layout
is inactive.

KV unavailability is a typed reason, not an empty estimate. Stable codes include
`model_not_parsed`, `missing_architecture`, `unsupported_architecture`,
`unsupported_attention_layout`, missing/unusable/zero dimensions,
KV-head inconsistencies, non-divisible head counts, unavailable head dimensions,
zero context or sequences, and `arithmetic_overflow`. See
[PLAN_SCHEMA.md](PLAN_SCHEMA.md#7-kv-cache-component) for the complete list.

### Worked KV example

For 32 layers, 32 query heads, 8 KV heads, default 128-element key/value widths,
one sequence, 4,096 context tokens, and F16:

```text
elements = 4096 * 32 * 8 * (128 + 128) * 1 = 268,435,456
bytes    = 268,435,456 * 2                 = 536,870,912 bytes (512 MiB)
```

Changing to `--kv-cache f32` doubles this to 1,073,741,824 bytes. Doubling
`--sequences` doubles the modeled cache. These are formula results, not measured
allocations.

## 7. Budgets, reserves, and comparison rules

An effective budget is:

```text
effective = available - reserve
```

The subtraction is allowed only when both values are known. If `available` or
`reserve` is unknown, `effective` is unknown and a reason is retained. If
`available <= reserve`, the effective budget is a non-negative derived zero,
`usable` is false, and the reason says that availability does not exceed the
reserve. There is no negative budget and no wrapping subtraction.

A component comparison is:

```text
modeled_components <= effective  -> within_budget = true, headroom = effective - modeled
modeled_components >  effective  -> within_budget = false, shortfall = modeled - effective
unknown input                    -> within_budget = null, neither headroom nor shortfall
```

An equal comparison is within budget with zero headroom. Component sums are
checked; an unknown component makes the sum unknown rather than contributing zero.
An overflow makes the sum unknown.

The default reserves are conservative product policy:

- 1,536 MiB host reserve;
- 1,024 MiB device reserve.

They are configurable and deliberately visible in both human and JSON output.
The device figure is informed by, but not identical to, the upstream llama.cpp
`--fit-target` default of 1024 MiB. The host figure is a policy choice for a
shared desktop, not a measured requirement. Neither number is universal.

Host availability is `MemAvailable` from `/proc/meminfo`. The planner never
substitutes `MemFree`. Device free memory is derived from `nvidia-smi` total minus
used because `nvidia-smi` does not report free memory directly. If either device
value is missing, free VRAM is unknown. There is no Vulkan memory observation and
no AMD/Intel/other-device budget in Stage 03.

## 8. Scenarios and qualified outcomes

Exactly two scenarios are evaluated, in deterministic order:

| ID | Placement | Host comparison | Device comparison |
| --- | --- | --- | --- |
| `cpu_weights_and_kv` | Weights and KV on host RAM | Weights + KV against host budget | None |
| `device_weights_cpu_kv` | Weights on the selected NVIDIA device; KV on host RAM | KV against host budget | Weights against device budget |

The second scenario is a hypothetical full-device weight placement. It does not
model partial offload, GPU-resident KV, a backend's actual tensor placement, or
GPU layer counts.

Outcomes are qualified:

| Outcome | Meaning |
| --- | --- |
| `estimated_components_within_budget` | Every comparison needed by the scenario has known values and is within its effective budget. |
| `estimated_components_exceed_budget` | At least one modeled comparison is known to exceed its effective budget. |
| `insufficient_data` | A required observation, estimate, device, or budget is unavailable and no comparison is known to exceed. |

Precedence is explicit: a known exceed wins over an unknown comparison; otherwise
unknown data produces `insufficient_data`; only then can a scenario be
`within_budget`. The device scenario is `insufficient_data` when no device was
observed or an explicit device index is absent, even if its host comparison is
within budget.

The plan never emits an unqualified `model_fits`, `launchable`, `supported`, or
`safe_to_run` result. It does not claim that launch success, engine support,
backend workspace, compute buffers, runtime overhead, or driver behavior were
verified. Those caveats are present in every plan and in each scenario.

## 9. Context upper bound

The optional context estimate is a budget upper bound, not a recommendation:

```text
remaining = host_effective_budget - file_size_weight_proxy
tokens    = floor(remaining / per_token_kv_bytes)
```

It is unavailable when weights or KV dimensions are unknown, the effective host
budget is unusable, the remaining budget is not positive, or arithmetic fails. The
value is capped at the planner's 1,048,576-token option limit and a warning records
the cap. It is not capped at the model's declared context length; instead, a
warning records when the estimate exceeds the declared value. The estimate excludes
runtime overhead and backend workspace and is not a model, engine, or backend
context limit.

The per-token bytes include every requested sequence and all modeled layers. A
larger value is not a promise that the model can allocate or process that context.

## 10. Warnings, unknowns, and partial plans

The plan retains:

- `warnings` for known limitations and inconsistent inputs;
- `unknowns` for unresolved quantities;
- `caveats` for scope limits that apply to the whole plan;
- per-scenario `reasons` and `caveats` for local scope.

Warnings are stable codes rather than free-form status. Examples include
`file_size_proxy`, `runtime_overhead_unmodeled`, `model_not_parsed`,
`metadata_incomplete`, `sharded_file_selected`, `sidecar_filename_hint`,
`unsupported_attention_layout`, `kv_cache_unavailable`,
`requested_context_exceeds_declared`, `host_observation_incomplete`,
`device_observation_incomplete`, `explicit_device_missing`,
`independent_sequences_assumption`, and reserve-exhaustion codes. The complete
list and field shapes are in [PLAN_SCHEMA.md](PLAN_SCHEMA.md).

A partial plan is a successful result. For example, a malformed file produces
`parse_status: malformed`, an observed file size, unknown weights and KV, and
`insufficient_data` for both scenarios. A missing `nvidia-smi` leaves the CPU
scenario potentially meaningful while the device scenario is `insufficient_data`.

## 11. Determinism and arithmetic

- Byte quantities are exact integers stored as `u64`; JSON byte values are plain
  integers. Human display may use binary units for readability.
- No floating-point value is used for canonical storage or arithmetic.
- Multiplications, additions, and subtractions are checked. Overflow produces an
  unknown value with a reason, not a panic or wrapped result.
- Resource snapshots are sorted by device index.
- Scenario order is fixed: CPU first, device second.
- Warnings and unknowns are de-duplicated by stable code.
- The only timestamps are observation and envelope generation times. A repeated
  command may produce a different timestamp and different current observations,
  but the model/options/component structure is stable for the same inputs.

## 12. Limitations and non-claims

The planner is intentionally conservative where metadata is insufficient, but it is
not a safety proof. In particular:

- File size is not exact resident weight memory.
- The KV formula is not backend allocation accounting.
- Runtime/backend overhead is always unknown in Stage 03.
- A device being visible does not prove a CUDA or Vulkan backend is installed,
  compatible, or able to launch the model.
- No real GGUF model, engine, GPU allocation, inference run, or benchmark was used
  to validate the estimates.
- No model cache, plan store, stable model ID, shard grouping, or engine adapter
  exists.
- No launch refusal or override rule is implemented because there is no launch
  path in Stage 03. A future runtime stage must add that policy with its own tests.

The planner's accepted Stage 03 validation is based on deterministic arithmetic
unit tests, synthetic GGUF fixtures, synthetic resource snapshots, CLI integration
tests, JSON round-trips, malformed-input tests, and docs checks. Real-model and
engine validation remain deferred.
