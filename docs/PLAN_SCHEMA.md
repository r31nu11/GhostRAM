# Memory plan JSON schema

**Status:** Implemented in Stage 03; source-only and pending formal acceptance.
There is no release artifact or installer.
**Schema identifier:** `ghostram.models.plan/1`.
**Implementation:** `crates/ghostram-planner/src/plan.rs`,
`crates/ghostram-planner/src/evidence.rs`, and the `models plan` command in
`crates/ghostram-cli`.
**Behavior:** [MEMORY_PLANNER.md](MEMORY_PLANNER.md).

`ghostram models plan --json` writes exactly one JSON object followed by a newline
to stdout. Human-readable text is not mixed into stdout. Usage and path errors go
to stderr with a non-zero exit code before a plan payload starts. An output-device
failure is also non-zero and can leave partial stdout; it is not a valid complete
envelope.

The plan is an ephemeral, versioned estimate envelope. It is not a model identity,
a measurement, a launch decision, or a compatibility claim. Paths and timestamps
are observations of one invocation; they are not stable identifiers.

## 1. Envelope

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | Always `ghostram.models.plan/1` for this build. |
| `application_version` | string | Version of the `ghostram` binary. Source builds currently report `0.1.0-dev`. |
| `generated_at` | string | UTC RFC 3339 generation time with whole-second precision, for example `2026-10-21T12:00:00Z`. |
| `plan` | object | Complete plan payload described below. |

Consumers must reject a different schema identifier rather than guessing at field
meanings. Unknown additional fields in this schema should be ignored. A change to
the meaning of an existing field, a removed field, or a type change requires a new
schema identifier. The layout of `ghostram.doctor/1` and
`ghostram.models.scan/1` is unchanged by this schema.

Byte counts are exact JSON integers, never rounded strings or floating-point
numbers. An unknown quantity is an object with `"evidence": "unknown"` and a
`reason`; it is not represented as zero, `null`, or a guessed value.

## 2. Evidence objects

Every scalar planner quantity, budget, and component is tagged with its evidence
class. Scalar options also carry evidence; `device_selection` is a tagged mode
object instead of an evidence wrapper:

```json
{"evidence": "observed", "value": 1610612736}
{"evidence": "explicit_input", "value": 8192}
{"evidence": "derived", "value": 1073741824, "reason": "available minus the configured reserve"}
{"evidence": "estimated", "value": 536870912, "reason": "uniform full-attention KV formula; ..."}
{"evidence": "product_default", "value": 4096, "reason": "4,096-token planning default; ..."}
{"evidence": "unknown", "reason": "MemAvailable was not reported"}
```

| Evidence class | Shape | Meaning |
| --- | --- | --- |
| `observed` | `value` | A current file or system observation. |
| `explicit_input` | `value` | A value supplied by the caller or command line. |
| `derived` | `value`, `reason` | Exact checked arithmetic from known values. |
| `estimated` | `value`, `reason` | Proxy, formula, or assumption-based estimate. |
| `product_default` | `value`, `reason` | Documented policy default chosen by this build. |
| `unknown` | `reason`, no `value` | No usable value is available. |

`value` may be a JSON integer, string, or another schema object depending on the
quantity. `reason` is explanatory text and is not a stable machine-readable code;
use `warnings[].code`, `unknowns[].code`, and typed fields for program logic.

## 3. Plan payload

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `model` | object | Always | Compact model summary from the selected file. |
| `options` | object | Always | Planning options and their evidence. |
| `resources` | object | Always | Host/NVIDIA observations used for this plan. |
| `selected_device_index` | integer | Omitted when no device was selected | Selected `nvidia-smi` index. |
| `components` | object | Always | Modeled weight, KV-cache, and runtime-overhead components. |
| `scenarios` | array | Always | Exactly two scenarios in deterministic order. |
| `context_estimate` | object | Always | Optional budget-limited context upper bound. |
| `unknowns` | array | Always | Unresolved quantities, de-duplicated by code. |
| `warnings` | array | Always | Planning warnings, de-duplicated by code. |
| `caveats` | array of strings | Always | Scope limits for the whole plan. |

## 4. Model summary

`plan.model` is derived from the selected `FileCatalogEntry`. It never includes
tokenizer values, merges, chat templates, prompts, tensor bytes, arbitrary metadata
blobs, or a content hash. The command does not hash the file.

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `path` | string | Always | Absolute, non-canonicalized selected path. |
| `file_size` | evidence integer | Always | Observed filesystem size, or an unknown reason. |
| `parse_status` | string | Always | `parsed`, `unsupported`, `malformed`, `unreadable`, or `changed`. |
| `failure` | object | Omitted when parsed | Structural failure detail from the catalog parser. |
| `gguf_version` | integer | Omitted unless metadata was parsed | GGUF version `2` or `3`. |
| `tensor_count` | integer | Omitted unless metadata was parsed | Declared tensor-descriptor count. Tensor contents were not parsed. |
| `metadata_state` | string | Omitted unless metadata was parsed | `available` or `incomplete`. |
| `architecture` | field object | Omitted unless present | `general.architecture` with provenance. |
| `name` | field object | Omitted unless present | `general.name` with provenance. |
| `quantization` | field object | Omitted unless present | Metadata quantization hint. It is not a tensor validation. |
| `declared_context_tokens` | field object | Omitted unless present | Parsed architecture context length. It is not a launch limit. |
| `filename_hints` | object | Always | Filename role/shard hints; may be `{}`. |
| `metadata_warnings` | array | Omitted when empty | Non-fatal catalog warnings copied from the parser. |

A **field object** is:

```json
{"value": "llama", "source": "explicit"}
```

`source` is `explicit`, `derived`, or `default`. A default means the parser or
planner applied a documented format default; it is still visible.

A **failure object** is:

```json
{
  "kind": "truncated",
  "message": "truncated metadata value",
  "offset": 128
}
```

`offset` is omitted when unknown. `kind` is one of `invalid_magic`,
`unsupported_version`, `unsupported_endianness`, `unsupported_metadata_type`,
`invalid_alignment`, `duplicate_metadata_key`, `duplicate_tensor_name`,
`invalid_tensor`, `truncated`, `malformed`, `limit_exceeded`, `io`, or `changed`.
These are the same catalog failure kinds documented in
[MODELS_SCHEMA.md](MODELS_SCHEMA.md); they are not engine acceptance results.

`filename_hints` uses the catalog shape:

```json
{"role": "lora", "shard": {"number": 2, "total": 4}}
```

Both members are optional; `{}` is valid. `role` is `mmproj`, `mtp`, `lora`, or
`vocab`. Filename hints are heuristics only. A shard hint makes total weight
unknown; a sidecar hint produces a warning and is not combined with a base model.

`metadata_warnings` entries have a stable `code` and a bounded `message`. Codes are
`missing_architecture`, `unexpected_metadata_type`, `oversized_metadata_value`, and
`unknown_tensor_type`.

## 5. Options

`plan.options` records the effective options, including whether each came from the
command line or a product default.

| Field | Type | Values |
| --- | --- | --- |
| `context_tokens` | evidence integer | Context per sequence. |
| `sequences` | evidence integer | Independent full-context sequences. |
| `kv_cache_type` | evidence string | `f16` or `f32`. |
| `ram_reserve` | evidence integer | Host reserve in bytes. |
| `vram_reserve` | evidence integer | Device reserve in bytes. |
| `device_selection` | object | `{"mode": "first_observed"}` or `{"mode": "explicit", "index": 0}`. |

The product defaults are 4,096 context tokens, 1 sequence, F16 KV, 1,536 MiB host
reserve, 1,024 MiB device reserve, and the first observed NVIDIA device. Explicit
values carry `explicit_input`. Defaults carry `product_default` and a reason.
Unknown or invalid options fail command validation and do not produce a plan.

## 6. Resources

`plan.resources` is a snapshot from the lightweight memory/NVIDIA path. It is not
the complete doctor report.

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `observed_at` | string | Always | UTC RFC 3339 start time of observation. |
| `host` | object | Always | Host memory observation. |
| `devices_state` | category object | Always | Availability of NVIDIA device collection. |
| `devices` | array | Always | Observed NVIDIA devices, sorted by index. May be empty. |

`host`:

| Field | Type | Meaning |
| --- | --- | --- |
| `state` | category object | Memory probe state. |
| `source` | string | Always `proc_meminfo`. |
| `total` | evidence integer | `MemTotal` in bytes, or unknown. |
| `available` | evidence integer | `MemAvailable` in bytes, or unknown. `MemFree` is never substituted. |

`devices_state` and `host.state` use:

```json
{"state": "available"}
{"state": "unavailable", "reason": "nvidia-smi was not found on PATH"}
```

`state` is `available`, `partial`, or `unavailable`; `reason` is optional.

Each `devices[]` item:

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `index` | integer | Always | `nvidia-smi` device index. |
| `name` | string | Always | Device product name. |
| `driver_version` | string | Optional | Driver version, when reported. |
| `compute_capability` | string | Optional | Compute capability, when reported. |
| `source` | string | Always | `nvidia_smi`. |
| `total` | evidence integer | Always | Total VRAM in bytes, or unknown. |
| `used` | evidence integer | Always | Used VRAM in bytes, or unknown. |
| `free` | evidence integer | Always | Derived total minus used, or unknown. |

`devices` is empty when no NVIDIA device was observed. There is no Vulkan, AMD,
Intel, or other-device memory model in this schema. A device observation is not a
backend availability or launch-success claim.

## 7. Components

`plan.components` contains only the modeled components:

| Field | Type | Meaning |
| --- | --- | --- |
| `model_weights` | weight object | File-size proxy or unavailable. |
| `kv_cache` | KV object | Uniform formula or unavailable. |
| `runtime_overhead` | evidence integer | Always unknown in Stage 03. |

### 7.1 Model weights

```json
{
  "basis": {"evidence": "observed", "value": 2362232012},
  "estimate": {
    "evidence": "estimated",
    "value": 2362232012,
    "reason": "low-confidence proxy: GGUF file size; ..."
  },
  "method": "gguf_file_size_proxy"
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `basis` | evidence integer | The observed file size when available, even if the estimate is unavailable. |
| `estimate` | evidence integer | Low-confidence file-size proxy, or unknown. |
| `method` | string | `gguf_file_size_proxy` or `unavailable`. |

The estimate is unavailable for non-parsed files, missing filesystem size, and
multi-shard filename hints. A shard warning is present when the filename suggests a
shard. The proxy does not model tensor allocation, alignment, quantization storage,
mmap residency, partial offload, or GPU layer counts.

### 7.2 KV-cache component

```json
{
  "estimate": {
    "evidence": "estimated",
    "value": 536870912,
    "reason": "uniform full-attention KV formula; ..."
  },
  "method": "uniform_full_attention",
  "profile": "llama_uniform_full_attention_v1",
  "dimensions": {
    "context_tokens": 4096,
    "sequences": 1,
    "layers": {"value": 32, "source": "explicit"},
    "head_count": {"value": 32, "source": "explicit"},
    "kv_heads": {"value": 8, "source": "explicit"},
    "key_dimension": {"value": 128, "source": "default"},
    "value_dimension": {"value": 128, "source": "default"},
    "cache_type": "f16",
    "elements": 268435456
  }
}
```

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `estimate` | evidence integer | Always | Estimated bytes, or unknown with a reason. |
| `method` | string | Always | `uniform_full_attention` or `unavailable`. |
| `profile` | string | Optional | `llama_uniform_full_attention_v1` when the architecture/profile was recognized. |
| `dimensions` | object | Optional | Audit dimensions when the formula succeeded. |
| `unavailable` | object | Optional | Typed code and message when no estimate was produced. |

`dimensions` fields are `context_tokens`, `sequences`, `layers`, `head_count`,
`kv_heads`, `key_dimension`, `value_dimension`, `cache_type`, and `elements`.
The five dimension fields `layers`, `head_count`, `kv_heads`, `key_dimension`, and
`value_dimension` are field objects with `value` and `source`; the others are
integers or the cache-type string. `elements` is the total K+V element count
across all modeled layers, sequences, and context tokens.

Typed KV unavailable codes:

| Code | Meaning |
| --- | --- |
| `model_not_parsed` | The selected file was not parsed as supported GGUF. |
| `missing_architecture` | `general.architecture` is absent or empty. |
| `unsupported_architecture` | The architecture is not `llama`. |
| `unsupported_attention_layout` | A recognized special attention-layout marker is present. |
| `missing_layer_count` | `llama.block_count` is absent. |
| `unusable_layer_count` | The layer count was present but not a usable scalar. |
| `zero_layer_count` | The layer count is zero. |
| `missing_head_count` | `llama.attention.head_count` is absent. |
| `unusable_head_count` | The query head count was present but not a usable scalar. |
| `zero_head_count` | The query head count is zero. |
| `missing_embedding_length` | `llama.embedding_length` is absent. |
| `unusable_embedding_length` | The embedding length was present but not a usable scalar. |
| `zero_embedding_length` | The embedding length is zero. |
| `unusable_kv_head_count` | The KV head count was present but not a usable scalar. |
| `zero_kv_head_count` | The KV head count is zero. |
| `kv_heads_exceed_head_count` | KV heads exceed query heads. |
| `head_count_not_divisible_by_kv_heads` | Query heads are not divisible by KV heads. |
| `unusable_key_length` | The key length was present but not a usable scalar. |
| `unusable_value_length` | The value length was present but not a usable scalar. |
| `zero_key_dimension` | The derived or explicit key width is zero. |
| `zero_value_dimension` | The derived or explicit value width is zero. |
| `head_dimension_unavailable` | A default head width cannot be derived. |
| `zero_context_tokens` | Context is zero. |
| `zero_sequences` | Sequence count is zero. |
| `arithmetic_overflow` | Checked element or byte arithmetic overflowed. |

An `unavailable` object is:

```json
{"code": "unsupported_attention_layout", "message": "..."}
```

The `message` is explanatory and is not a stable contract. Use `code`.

### 7.3 Runtime overhead

`runtime_overhead` is always an unknown evidence object in Stage 03:

```json
{
  "evidence": "unknown",
  "reason": "backend workspace, compute buffers, graph allocations, and driver runtime overhead are not modeled"
}
```

No scenario includes this quantity. A within-budget outcome therefore means only
that the modeled weights and KV cache are within the configured budgets.

## 8. Budgets and scenarios

Each scenario may contain a `host` and/or `device` comparison object:

```json
{
  "budget": {
    "available": {"evidence": "observed", "value": 8589934592},
    "reserve": {
      "evidence": "product_default",
      "value": 1610612736,
      "reason": "1,536 MiB host reserve; ..."
    },
    "effective": {
      "evidence": "derived",
      "value": 6979321856,
      "reason": "available minus the configured reserve"
    },
    "usable": true
  },
  "estimated_components": {
    "evidence": "estimated",
    "value": 2899102924,
    "reason": "sum of modeled model weights and KV cache for the CPU scenario"
  },
  "within_budget": true,
  "headroom": {
    "evidence": "derived",
    "value": 4080218932,
    "reason": "effective budget minus modeled components"
  }
}
```

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `budget.available` | evidence integer | Always | Observed/derived available quantity. |
| `budget.reserve` | evidence integer | Always | Configured reserve. |
| `budget.effective` | evidence integer | Always | Available minus reserve, or unknown. |
| `budget.usable` | boolean | Always | True only for a positive effective budget. |
| `budget.reason` | string | Omitted when usable | Why the budget is unusable. |
| `estimated_components` | evidence integer | Always | Component sum assigned to that resource. |
| `within_budget` | boolean or `null` | Always | `null` when either side is unknown. |
| `headroom` | evidence integer | Optional | Effective minus components when within budget. |
| `shortfall` | evidence integer | Optional | Components minus effective when over budget. |

If `available <= reserve`, `effective` is a derived zero and `usable` is false.
No negative effective value exists. `headroom` and `shortfall` are mutually
exclusive and are omitted when the comparison is unknown.

`plan.scenarios` contains exactly two entries in this order:

| `id` | `description` | Comparisons |
| --- | --- | --- |
| `cpu_weights_and_kv` | Model weights and KV cache on host RAM. | `host` only. |
| `device_weights_cpu_kv` | Weights on selected NVIDIA device; KV on host RAM. | `host` and, when a device was selected, `device`. |

Each scenario:

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `id` | string | Always | Scenario identifier. |
| `description` | string | Always | Human-readable placement description. |
| `host` | comparison | Optional | Host comparison, when applicable. |
| `device` | comparison | Optional | Device comparison when a device was selected. |
| `outcome` | string | Always | Qualified outcome. |
| `reasons` | array of strings | Omitted when empty | Why the outcome is not within budget. |
| `caveats` | array of strings | Always | Scenario scope limits. |

Outcome values:

| Outcome | Meaning |
| --- | --- |
| `estimated_components_within_budget` | Every required modeled comparison is known and within budget. |
| `estimated_components_exceed_budget` | At least one required modeled comparison is known to exceed budget. |
| `insufficient_data` | A required estimate/observation/device/budget is unknown and no known exceed exists. |

A known exceed takes precedence over unknown data. The device scenario is
`insufficient_data` when no device was observed or the explicit device index is
missing. This outcome vocabulary deliberately contains no unqualified
`model_fits`, `launchable`, `supported`, or `safe_to_run` value.

## 9. Context estimate

`plan.context_estimate`:

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `estimate` | evidence integer | Always | Budget-limited token upper bound, or unknown. |
| `method` | string | Always | `host_budget_after_weight_proxy` or `unavailable`. |
| `capped_at_limit` | boolean | Always | Whether the planner's 1,048,576-token cap was applied. |
| `note` | string | Optional | Scope note. |

The formula is `floor((host_effective - weight_proxy) / per_token_kv_bytes)`. It is
not a recommendation, a declared model limit, an engine limit, or a validated
context. Runtime overhead and backend workspace are excluded. A value above the
declared GGUF context produces `context_estimate_exceeds_declared`; it is not
silently clamped to the declared value.

## 10. Unknowns

`plan.unknowns[]`:

```json
{"code": "runtime_overhead", "description": "..."}
```

| Code | Meaning |
| --- | --- |
| `runtime_overhead` | Backend workspace, compute buffers, graph allocations, and driver overhead are not modeled. |
| `model_weights` | No weight estimate is available. |
| `kv_cache` | No KV-cache estimate is available. |
| `host_available_memory` | `MemAvailable` is unavailable. |
| `device_free_memory` | No selected device or free-memory value is available. |

Unknowns are de-duplicated by code. They are explanatory; the component objects
remain the authoritative source of unknown values.

## 11. Warnings

`plan.warnings[]`:

```json
{"code": "file_size_proxy", "message": "..."}
```

| Code | Meaning |
| --- | --- |
| `file_size_proxy` | Weights use the GGUF file size as a low-confidence proxy. |
| `runtime_overhead_unmodeled` | Runtime/backend overhead is excluded from every scenario. |
| `model_not_parsed` | The model was not parsed; parsed metadata is unavailable. |
| `metadata_incomplete` | Required descriptive metadata is incomplete. |
| `sharded_file_selected` | The filename suggests a shard of a multi-file model. |
| `sidecar_filename_hint` | The filename suggests a sidecar or adapter. |
| `unsupported_attention_layout` | Special attention-layout metadata is present. |
| `kv_cache_unavailable` | The KV estimate is unavailable; the message includes the typed reason. |
| `requested_context_exceeds_declared` | Requested context exceeds the GGUF declared value. |
| `host_observation_incomplete` | Host memory observation is partial or unavailable. |
| `device_observation_incomplete` | NVIDIA device observation is partial, unavailable, or empty. |
| `multiple_devices_observed` | More than one NVIDIA device was observed; only one is planned. |
| `explicit_device_missing` | The explicitly requested device index was not observed. |
| `independent_sequences_assumption` | Multiple sequences are modeled as independent full contexts. |
| `host_budget_exhausted_by_reserve` | Available host memory does not exceed the reserve. |
| `device_budget_exhausted_by_reserve` | Free device memory does not exceed the reserve. |
| `context_estimate_capped` | The context upper bound was capped at the planner limit. |
| `context_estimate_exceeds_declared` | The context upper bound exceeds the declared GGUF context. |
| `file_size_unavailable` | The filesystem size needed for the weight proxy is unavailable. |

Messages are explanatory and may change; codes are the stable machine contract.
Warnings are de-duplicated by code.

## 12. Caveats

`plan.caveats` is an array of strings covering the whole plan. Every plan includes
statements that:

- estimates are not measurements;
- no engine was launched and no benchmark was run;
- an outcome does not predict load, launch, or safe operation;
- only modeled weights and KV cache are compared;
- launch success, engine support, backend workspace, and runtime overhead are
  unverified;
- file size is a low-confidence weight proxy;
- KV estimates are uniform full-attention formula results;
- device presence does not prove a CUDA or Vulkan backend is installed or
  compatible.

Caveats are not optional disclaimers that a consumer may drop; they describe the
scope of the numeric fields. Their text is not a stable machine-readable schema.

## 13. Representative example

The following is a shortened but valid `ghostram.models.plan/1` example from a
synthetic file. Some optional fields and array entries are omitted to keep the
example readable; the tables above are authoritative. The `generated_at`,
resource values, and context bound vary by host and invocation.

```json
{
  "schema_version": "ghostram.models.plan/1",
  "application_version": "0.1.0-dev",
  "generated_at": "2026-10-21T12:00:00Z",
  "plan": {
    "model": {
      "path": "/models/example.gguf",
      "file_size": {"evidence": "observed", "value": 453},
      "parse_status": "parsed",
      "gguf_version": 3,
      "tensor_count": 0,
      "metadata_state": "available",
      "architecture": {"value": "llama", "source": "explicit"},
      "name": {"value": "Documentation Example", "source": "explicit"},
      "quantization": {"value": "Q4_K_M", "source": "explicit"},
      "declared_context_tokens": {"value": 8192, "source": "explicit"},
      "filename_hints": {}
    },
    "options": {
      "context_tokens": {
        "evidence": "product_default",
        "value": 4096,
        "reason": "4,096-token planning default; not a model or engine maximum"
      },
      "sequences": {
        "evidence": "product_default",
        "value": 1,
        "reason": "one independent sequence planning default"
      },
      "kv_cache_type": {
        "evidence": "product_default",
        "value": "f16",
        "reason": "F16 KV-cache product default; F32 is supported, quantized KV caches are not modeled"
      },
      "ram_reserve": {
        "evidence": "product_default",
        "value": 1610612736,
        "reason": "1,536 MiB host reserve; conservative product policy, not a universal requirement"
      },
      "vram_reserve": {
        "evidence": "product_default",
        "value": 1073741824,
        "reason": "1,024 MiB device reserve; conservative product policy, not a universal requirement"
      },
      "device_selection": {"mode": "first_observed"}
    },
    "resources": {
      "observed_at": "2026-10-21T12:00:00Z",
      "host": {
        "state": {"state": "available"},
        "source": "proc_meminfo",
        "total": {"evidence": "observed", "value": 7989460992},
        "available": {"evidence": "observed", "value": 4022394880}
      },
      "devices_state": {
        "state": "unavailable",
        "reason": "nvidia-smi was not found on PATH"
      },
      "devices": []
    },
    "components": {
      "model_weights": {
        "basis": {"evidence": "observed", "value": 453},
        "estimate": {
          "evidence": "estimated",
          "value": 453,
          "reason": "low-confidence proxy: GGUF file size; exact resident tensor allocation, alignment padding, quantization storage, and mmap behavior are not modeled"
        },
        "method": "gguf_file_size_proxy"
      },
      "kv_cache": {
        "estimate": {
          "evidence": "estimated",
          "value": 536870912,
          "reason": "uniform full-attention KV formula; backend padding, alignment, sliding windows, and workspace are not modeled"
        },
        "method": "uniform_full_attention",
        "profile": "llama_uniform_full_attention_v1",
        "dimensions": {
          "context_tokens": 4096,
          "sequences": 1,
          "layers": {"value": 32, "source": "explicit"},
          "head_count": {"value": 32, "source": "explicit"},
          "kv_heads": {"value": 8, "source": "explicit"},
          "key_dimension": {"value": 128, "source": "default"},
          "value_dimension": {"value": 128, "source": "default"},
          "cache_type": "f16",
          "elements": 268435456
        }
      },
      "runtime_overhead": {
        "evidence": "unknown",
        "reason": "backend workspace, compute buffers, graph allocations, and driver runtime overhead are not modeled"
      }
    },
    "scenarios": [
      {
        "id": "cpu_weights_and_kv",
        "description": "Model weights and KV cache are assigned to host RAM.",
        "host": {
          "budget": {
            "available": {"evidence": "observed", "value": 4022394880},
            "reserve": {
              "evidence": "product_default",
              "value": 1610612736,
              "reason": "1,536 MiB host reserve; conservative product policy, not a universal requirement"
            },
            "effective": {
              "evidence": "derived",
              "value": 2411782144,
              "reason": "available minus the configured reserve"
            },
            "usable": true
          },
          "estimated_components": {
            "evidence": "estimated",
            "value": 536871365,
            "reason": "sum of modeled model weights and KV cache for the CPU scenario"
          },
          "within_budget": true,
          "headroom": {
            "evidence": "derived",
            "value": 1874910779,
            "reason": "effective budget minus modeled components"
          }
        },
        "outcome": "estimated_components_within_budget",
        "caveats": [
          "Weights and KV cache are treated as host-resident.",
          "Only modeled weights and KV cache are compared; launch success, engine support, backend workspace, and runtime overhead are unverified."
        ]
      },
      {
        "id": "device_weights_cpu_kv",
        "description": "Model weights are assigned to the selected NVIDIA device; KV cache is assigned to host RAM.",
        "host": {
          "budget": {
            "available": {"evidence": "observed", "value": 4022394880},
            "reserve": {
              "evidence": "product_default",
              "value": 1610612736,
              "reason": "1,536 MiB host reserve; conservative product policy, not a universal requirement"
            },
            "effective": {
              "evidence": "derived",
              "value": 2411782144,
              "reason": "available minus the configured reserve"
            },
            "usable": true
          },
          "estimated_components": {
            "evidence": "estimated",
            "value": 536870912,
            "reason": "uniform full-attention KV formula; backend padding, alignment, sliding windows, and workspace are not modeled"
          },
          "within_budget": true,
          "headroom": {
            "evidence": "derived",
            "value": 1874911232,
            "reason": "effective budget minus modeled components"
          }
        },
        "outcome": "insufficient_data",
        "reasons": [
          "no NVIDIA device was selected or observed, so device weights cannot be compared"
        ],
        "caveats": [
          "Weights are treated as fully device-resident; KV cache is treated as host-resident. Partial offload is not modeled.",
          "Only modeled weights and KV cache are compared; launch success, engine support, backend workspace, and runtime overhead are unverified."
        ]
      }
    ],
    "context_estimate": {
      "estimate": {
        "evidence": "estimated",
        "value": 18400,
        "reason": "upper bound from host effective budget after the file-size weight proxy; not a recommended or validated context, and runtime overhead is excluded"
      },
      "method": "host_budget_after_weight_proxy",
      "capped_at_limit": false,
      "note": "This is a budget upper bound, not a model, engine, or backend context limit."
    },
    "unknowns": [
      {
        "code": "runtime_overhead",
        "description": "backend workspace, compute buffers, graph allocations, and driver runtime overhead are not modeled"
      },
      {
        "code": "device_free_memory",
        "description": "no NVIDIA device was selected or observed"
      }
    ],
    "warnings": [
      {
        "code": "file_size_proxy",
        "message": "model weights use GGUF file size as a low-confidence proxy; exact resident tensor allocation, alignment padding, quantization storage, and mmap behavior are not modeled"
      },
      {
        "code": "device_observation_incomplete",
        "message": "NVIDIA device observation is unavailable: no NVIDIA device was reported"
      },
      {
        "code": "runtime_overhead_unmodeled",
        "message": "backend workspace, compute buffers, and driver runtime overhead are excluded from every scenario"
      }
    ],
    "caveats": [
      "Planning estimates are not measurements; no inference engine was launched and no benchmark was run.",
      "A scenario outcome does not predict that a model will load, launch, or run safely.",
      "Only modeled weights and KV cache are compared; launch success, engine support, backend workspace, and runtime overhead are unverified."
    ]
  }
}
```

The example intentionally omits `selected_device_index` because no device was
observed. A real plan may also include a `failure`, catalog warnings, an explicit
device, a device comparison, shortfalls, or additional warnings and unknowns.

## 14. Compatibility and privacy rules

- The schema identifier is `ghostram.models.plan/1`. A consumer must reject an
  unknown identifier and should ignore unknown fields in this schema.
- Optional fields are omitted, not set to `null`, unless this document explicitly
  specifies a nullable field. `within_budget` is the only nullable boolean.
- Evidence tags are part of the machine contract; a consumer must not treat an
  estimate as a measurement or an unknown as zero.
- Byte values are integers. Human units are presentation only.
- Ordering is deterministic for scenarios and device records. Warning, unknown,
  and metadata-warning order follows deterministic construction but should not be
  used as a semantic key; use the code.
- Messages, descriptions, and caveat strings are explanatory text, not stable
  identifiers.
- Paths are observations, not stable model IDs. The command does not hash files.
- Plans are ephemeral. Nothing is persisted, and there is no cache or database.
- No tokenizer values, chat templates, prompts, tensor bytes, secrets, API keys, or
  arbitrary metadata dumps are included.
- The schema does not include an engine build, backend capability, launch result,
  process state, measurement, or benchmark result. Those belong to later stages.
- `ghostram.doctor/1` and `ghostram.models.scan/1` keep their existing contracts.

## 15. Related documents

- [Memory planner behavior](MEMORY_PLANNER.md)
- [KV-cache planning research](research/2026-10-21-kv-cache-planning.md)
- [Model scan schema](MODELS_SCHEMA.md)
- [GGUF catalog behavior](GGUF_CATALOG.md)
- [Engine support](ENGINE_SUPPORT.md)
- [Testing](TESTING.md)
