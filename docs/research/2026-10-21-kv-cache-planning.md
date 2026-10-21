# KV-cache planning research — 2026-10-21

**Research date:** 2026-10-21 (UTC).
**Status:** Stage 03 formula evidence. This is a snapshot of mutable upstream
sources, not a permanent compatibility promise or an engine-support claim.
**Related:** [Memory planner](../MEMORY_PLANNER.md),
[plan schema](../PLAN_SCHEMA.md), [engine support](../ENGINE_SUPPORT.md), and the
[Stage 03 report](../../reports/stages/stage-03-memory-planner.md).

## 1. Scope and method

The Stage 03 planner needed a defensible first KV-cache formula that could be
implemented without a model loader, an inference engine, or a backend allocation
experiment. The research asked:

1. How does upstream llama.cpp size the K and V cache tensors for a llama-family
   transformer with uniform full attention?
2. Which GGUF metadata keys supply the dimensions?
3. Which defaults and special-case layouts change the answer?
4. Which parts of resident memory the formula does **not** cover?

Upstream sources were read on 2026-10-21 from the mutable `master` branches and are
cited below at the reviewed commits. The URLs are pinned so a later reader can
compare the snapshot rather than a changed branch.

| Source | Reviewed commit | Pinned URL |
| --- | --- | --- |
| llama.cpp cache implementation | `8baabd53b77d1c48c6ef8d3b620436292f3f6231` | <https://github.com/ggml-org/llama.cpp/blob/8baabd53b77d1c48c6ef8d3b620436292f3f6231/src/llama-kv-cache.cpp> |
| llama.cpp hyperparameters | `8baabd53b77d1c48c6ef8d3b620436292f3f6231` | <https://github.com/ggml-org/llama.cpp/blob/8baabd53b77d1c48c6ef8d3b620436292f3f6231/src/llama-hparams.cpp> |
| llama.cpp model metadata loading | `8baabd53b77d1c48c6ef8d3b620436292f3f6231` | <https://github.com/ggml-org/llama.cpp/blob/8baabd53b77d1c48c6ef8d3b620436292f3f6231/src/llama-model.cpp> |
| llama.cpp server option reference | `8baabd53b77d1c48c6ef8d3b620436292f3f6231` | <https://github.com/ggml-org/llama.cpp/blob/8baabd53b77d1c48c6ef8d3b620436292f3f6231/tools/server/README.md> |
| ggml GGUF specification | `da8e2a54b919557b816b1c2beb432ccb766f7ce0` | <https://github.com/ggml-org/ggml/blob/da8e2a54b919557b816b1c2beb432ccb766f7ce0/docs/gguf.md> |

The Stage 02 parser already reads the relevant GGUF values. This note records the
formula and its boundaries; it does not add a second parser or a loader.

## 2. Observed upstream shape

In the reviewed `llama-kv-cache.cpp`:

- K and V tensors are created with three dimensions:
  `ggml_new_tensor_3d(ctx, type_k, n_embd_k_gqa, kv_size, n_stream)` and the same
  shape for V.
- `n_embd_k_gqa` and `n_embd_v_gqa` are the per-layer widths of the grouped
  key/value projections.
- The cache has one K/V pair per transformer layer, plus backend allocation
  overhead that is not represented by the tensor shape alone.

In the reviewed `llama-hparams.cpp`:

- `n_embd_k_gqa(il) = n_embd_head_k(il) * n_head_kv(il)`.
- `n_embd_v_gqa(il) = n_embd_head_v(il) * n_head_kv(il)`.
- `n_head_kv` defaults to `n_head` when the GGUF key is absent.
- `n_embd_head_k` and `n_embd_head_v` default to `n_embd / n_head` when the
  explicit key lengths are absent.

The resulting **uniform full-attention element count** used by GhostRAM is:

```text
elements = context_tokens
         * layers
         * kv_heads
         * (key_dimension + value_dimension)
         * sequences
bytes    = elements * bytes_per_cache_element
```

`F16` is 2 bytes per element and `F32` is 4 bytes per element. The formula is
implemented with checked `u64` arithmetic. It models each requested sequence as
holding the full context on every transformer layer.

### Worked example

For a `llama` model with 32 layers, 32 query heads, 8 KV heads, 4,096 embedding
elements, default 128-element key/value widths, one sequence, and 4,096 context
tokens:

```text
elements = 4096 * 32 * 8 * (128 + 128) * 1 = 268,435,456
bytes    = 268,435,456 * 2                 = 536,870,912 (512 MiB, F16)
```

The dimensions and their provenance are retained in the plan so a reader can
audit the result. A defaulted dimension is labeled `default`, not `explicit`.

## 3. Why the profile is narrow

The reviewed upstream implementation supports more layouts than the Stage 03
planner can defend from metadata alone:

- sliding-window attention can change the effective cache length per layer;
- sliding-window layer patterns can mix layer types;
- separate sliding-window key/value widths can change the projection width;
- latent attention and other architectures use different cache structures;
- per-layer arrays, GQA/MQA edge cases, and non-uniform layouts need validation;
- quantized cache types have backend-specific storage and quantization rules;
- padded, aligned, or backend workspace allocations are not described by the
  tensor dimensions alone.

Therefore Stage 03 supports exactly one documented architecture profile:

```text
profile: llama_uniform_full_attention_v1
architecture: llama
attention: uniform full attention on every modeled layer
```

The following are explicit `kv_cache_unavailable` conditions, not zero-byte
substitutes:

- the file was not parsed;
- `general.architecture` is absent or not `llama`;
- a recognized attention-layout marker is present, even if its value appears
  disabled: `attention.sliding_window`, `attention.sliding_window_pattern`,
  `attention.key_length_swa`, or `attention.value_length_swa`;
- a required dimension is absent, non-scalar, zero, inconsistent, or causes
  checked arithmetic overflow.

The parser records marker presence separately from the serialized
`ghostram.models.scan/1` metadata. That planning marker is not a new scan field and
does not change the Stage 02 JSON contract.

## 4. Cache types and defaults

The upstream server reference lists `f32`, `f16`, `bf16`, and several quantized KV
cache types, with `f16` as the default for `-ctk` and `-ctv`. GhostRAM deliberately
models only:

| Type | Bytes per element | Status |
| --- | --- | --- |
| `f16` | 2 | Supported and the visible product default. |
| `f32` | 4 | Supported as an explicit alternative. |

Quantized KV caches are **not** modeled. A quantized cache type is not a simple
linear byte multiplier in this planner: block formats, scale metadata, alignment,
backend layout, and kernel requirements are engine- and build-specific. Treating a
quantized cache as "F16 divided by N" would produce a false precision.

The 1,024 MiB device reserve product default is informed by, but not identical to,
the upstream `--fit-target` default of 1024 MiB. The host reserve is a separate
conservative product policy. Neither reserve is a universal requirement or a
measurement of this laptop.

## 5. Confidence and non-claims

| Claim | Confidence | Basis |
| --- | --- | --- |
| Upstream computes grouped K/V widths as head width times KV-head count. | Verified upstream | Pinned `llama-hparams.cpp` and `llama-kv-cache.cpp`. |
| Missing KV-head metadata defaults to the query-head count upstream. | Verified upstream | Pinned `llama-hparams.cpp`. |
| Missing key/value width defaults to embedding length divided by head count upstream. | Verified upstream | Pinned `llama-hparams.cpp`. |
| F16/F32 cache elements are 2/4 bytes. | Verified upstream | Upstream type sizes and the formula; not measured by GhostRAM. |
| The GhostRAM formula matches the engine's actual allocation for every model. | **Not verified** | No real GGUF model, engine, backend, or allocation observation was available. |
| A model will load, launch, or run within a budget. | **Not claimed** | The planner compares only modeled weights and KV cache. |
| Sliding-window, latent, quantized, or non-uniform layouts are supported. | **Not claimed** | These produce an explicit unavailable KV result. |
| GPU layer count, tensor placement, or exact resident weights can be inferred from file size. | **Not claimed** | File size is only a low-confidence proxy. |

No inference, engine installation, GPU computation, real-model parse, benchmark, or
allocation measurement was performed for this research. The Stage 03 tests use
deterministic synthetic GGUF metadata and synthetic resource snapshots.

## 6. Open questions

1. Does a real `llama` GGUF with explicit key/value widths produce the same
   dimensions as the planner's audit record?
2. How much do backend padding, alignment, and workspace add on the primary 4 GiB
   Pascal GPU for a small model?
3. Which additional architecture profiles can be supported without inventing
   layouts or silently treating arrays as scalars?
4. Can a future stage validate quantized KV storage against a pinned engine build
   and a measured cache allocation?
5. Does the engine's actual per-sequence cache sharing match the planner's
   independent-full-context upper bound?

These questions require a separately approved real-model/engine validation. They
are not hidden Stage 03 acceptance criteria, and this note does not imply that any
of them has been answered.
