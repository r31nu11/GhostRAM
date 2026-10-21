# GGUF catalog format research — 2026-10-21

**Research date:** 2026-10-21 (UTC).
**Status:** Stage 02 format evidence. This is a snapshot of mutable upstream files,
not a permanent compatibility promise.
**Related:** [GGUF catalog behavior](../GGUF_CATALOG.md),
[model scan schema](../MODELS_SCHEMA.md), [roadmap](../ROADMAP.md), and the
[Stage 02 report](../../reports/stages/stage-02-gguf-model-catalog.md).

## 1. Scope and method

The research asked what a read-only GGUF catalog must understand to extract safe,
compact metadata without implementing a model loader. It covered the file header,
metadata types, alignment, tokenizer field types, file-type labels, tensor
descriptors, and current tensor type identifiers.

Upstream sources were read on 2026-10-21 from the mutable `master` branches:

| Source | URL |
| --- | --- |
| GGUF specification | `https://github.com/ggml-org/ggml/blob/master/docs/gguf.md` |
| GGUF public header | `https://github.com/ggml-org/llama.cpp/blob/master/ggml/include/gguf.h` |
| GGUF loader/writer implementation | `https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/gguf.cpp` |
| GGML type definitions | `https://github.com/ggml-org/llama.cpp/blob/master/ggml/include/ggml.h` |
| Upstream Python constants | `https://github.com/ggml-org/llama.cpp/blob/master/gguf-py/gguf/constants.py` |

No exact upstream commit was recorded. The files were retrieved from their raw
GitHub equivalents on the research date. Because these branches move, a future
reader must re-check the sources rather than treating this note as a pinned spec.

No model, engine, GPU, or inference runtime was involved. No real GGUF file was
downloaded or opened. The evidence below is source/specification evidence; the
implementation was exercised only with synthetic fixtures.

## 2. File structure and bounds

**Confidence: Verified upstream.** The specification describes a magic number
`GGUF`, a `uint32` version, a `uint64` tensor count, a `uint64` metadata key/value
count, followed by metadata then tensor descriptors. Strings are a `uint64` length
followed by that many bytes and no terminating NUL. The current specification
describes version 3; its changelog records that version 2 changed most count
fields from `uint32` to `uint64`, and version 3 added big-endian support.

**Confidence: Verified by upstream loader inspection.** The current loader rejects
version 0 and version 1 and accepts versions up to the compiled `GGUF_VERSION` (3). GhostRAM deliberately supports only little-endian
v2/v3. Supporting v2 is useful for existing files; v1 has different structural
expectations and is explicitly rejected.

The specification says GGUF is little-endian by default but may be big-endian, and
that there is no fully reliable byte-order marker. The loader uses the version
word as a heuristic: a version whose low 16 bits are zero is treated as a
host/model endianness mismatch. GhostRAM uses a narrower version-byte reversal
check to report `unsupported_endianness` for big-endian v2/v3 headers and rejects
all big-endian files rather than implementing byte-swapped metadata and payload
handling.

## 3. Metadata value types

**Confidence: Verified upstream.** `gguf.h` defines these metadata type codes:

| Code | Type |
| --- | --- |
| 0 | `uint8` |
| 1 | `int8` |
| 2 | `uint16` |
| 3 | `int16` |
| 4 | `uint32` |
| 5 | `int32` |
| 6 | `float32` |
| 7 | `bool` |
| 8 | `string` |
| 9 | `array` |
| 10 | `uint64` |
| 11 | `int64` |
| 12 | `float64` |

Arrays carry an element type and a `uint64` count. The specification permits
nested arrays. GGUF booleans are expected to be false or true; GhostRAM rejects
values other than 0 or 1 in scalar and array metadata instead of treating every
nonzero value as true.

**Confidence: Verified upstream.** The specification states metadata keys are
ASCII, hierarchical, lower-snake-case segments separated by dots, and at most
65535 bytes. The current loader validates empty keys and duplicates but does not
appear to enforce the full key grammar in its read path.

**Decision:** GhostRAM validates key length and UTF-8, rejects empty and duplicate
keys, and does not enforce the upstream key grammar. Accepting a superset of key
spellings is harmless for extraction because only exact known keys are
interpreted, and it avoids rejecting a file solely because an unknown key used a
hyphen or uppercase letter. This is a documented relaxation, not full
conformance.

## 4. Alignment and tensor descriptors

**Confidence: Verified upstream.** `GGUF_DEFAULT_ALIGNMENT` is 32. The
specification says `general.alignment`, when present, is `uint32` and must be a
multiple of 8. Tensor names are strings of at most 64 bytes, tensors currently
have at most 4 dimensions, and each tensor-data offset is relative to the start of
the aligned tensor-data region and must itself be a multiple of the alignment.

**Confidence: Verified upstream implementation difference.** The current
`gguf.cpp` loader requires an explicit alignment to be a power of two, which is
stricter than the specification's multiple-of-8 rule. The same loader also checks
that tensor offsets are contiguous and computes/validates tensor payload sizes;
the catalog does not need those checks to extract metadata and deliberately does
not claim them.

**Decision:** GhostRAM follows the specification: an explicit alignment must be
`uint32`, nonzero, and a multiple of 8. It does not impose the loader's
power-of-two restriction. It checks that every tensor start offset is aligned and
lies within the available tensor-data region, but it does not verify contiguous
offsets, complete tensor ranges, overlap, quantization block size, or tensor
contents. Those are loader concerns and are deferred until an engine integration
stage.

The current upstream loader checks `name.length() >= GGML_MAX_NAME` (with
`GGML_MAX_NAME` equal to 64), effectively requiring fewer than 64 bytes. GhostRAM
follows the specification's "at most 64 bytes" wording and accepts exactly 64.
This one-byte boundary difference is recorded rather than hidden.

## 5. File-type labels

**Confidence: Verified upstream.** The specification and Python constants define
`general.file_type` labels from 0 through 18. Codes 5 and 6 are marked as removed
support but remain recognized historical labels. The catalog uses the reviewed
mapping below and does not guess at unknown codes:

| Code | Label |
| --- | --- |
| 0 | `ALL_F32` |
| 1 | `MOSTLY_F16` |
| 2 | `MOSTLY_Q4_0` |
| 3 | `MOSTLY_Q4_1` |
| 4 | `MOSTLY_Q4_1_SOME_F16` |
| 5 | `MOSTLY_Q4_2` |
| 6 | `MOSTLY_Q4_3` |
| 7 | `MOSTLY_Q8_0` |
| 8 | `MOSTLY_Q5_0` |
| 9 | `MOSTLY_Q5_1` |
| 10 | `MOSTLY_Q2_K` |
| 11 | `MOSTLY_Q3_K_S` |
| 12 | `MOSTLY_Q3_K_M` |
| 13 | `MOSTLY_Q3_K_L` |
| 14 | `MOSTLY_Q4_K_S` |
| 15 | `MOSTLY_Q4_K_M` |
| 16 | `MOSTLY_Q5_K_S` |
| 17 | `MOSTLY_Q5_K_M` |
| 18 | `MOSTLY_Q6_K` |

`general.quantization_version` is a `uint32`. `general.file_type` is optional and
can be inferred from tensor types. GhostRAM treats an explicit recognized label as
`explicit` and a fallback from the most common reviewed tensor type as `derived`;
the result is a catalog hint, not a guarantee that a loader will accept the model.

## 6. Tensor type identifiers

**Confidence: Verified upstream.** Current `ggml.h` defines tensor type IDs through
`GGML_TYPE_Q2_0 = 42` and `GGML_TYPE_COUNT = 43`. IDs that this catalog recognizes
are:

`0 F32`, `1 F16`, `2 Q4_0`, `3 Q4_1`, `6 Q5_0`, `7 Q5_1`, `8 Q8_0`, `9 Q8_1`,
`10 Q2_K`, `11 Q3_K`, `12 Q4_K`, `13 Q5_K`, `14 Q6_K`, `15 Q8_K`, `16 IQ2_XXS`,
`17 IQ2_XS`, `18 IQ3_XXS`, `19 IQ1_S`, `20 IQ4_NL`, `21 IQ3_S`, `22 IQ2_S`,
`23 IQ4_XS`, `24 I8`, `25 I16`, `26 I32`, `27 I64`, `28 F64`, `29 IQ1_M`,
`30 BF16`, `34 TQ1_0`, `35 TQ2_0`, `39 MXFP4`, `40 NVFP4`, `41 Q1_0`, `42 Q2_0`.

The gaps reflect removed, reserved, or never-assigned IDs. GhostRAM preserves an
unknown ID and descriptor count, emits no invented name, and emits an
`unknown_tensor_type` warning. Presence of a recognized type means only that the
catalog can name the identifier. It does not mean the development host, a backend,
or an engine supports it.

## 7. Tokenizer and common metadata

**Confidence: Verified upstream.** The current specification lists:

- `tokenizer.ggml.tokens: array[string]`
- `tokenizer.ggml.scores: array[float32]`
- `tokenizer.ggml.token_type: array[int32]`
- `tokenizer.ggml.merges: array[string]`
- `tokenizer.ggml.added_tokens: array[string]`
- chat templates and Hugging Face tokenizer JSON as string values whose content is
  tool-only for this stage.

An earlier local expectation treated `added_tokens` as an integer array. Upstream
source and the regression fixture confirm it is a string array. The parser now
uses `string[]` and summarises only element type and count.

Common architecture fields such as `context_length`, `embedding_length`,
`block_count`, `feed_forward_length`, attention counts, expert counts, and RoPE
dimension count are unsigned integer metadata in the specification. The catalog
retains a small common subset and skips all tokenizer values, chat-template text,
and arbitrary string content.

## 8. Decisions and confidence summary

| Decision | Evidence | Confidence | Consequence |
| --- | --- | --- | --- |
| Parse GGUF v2/v3 little-endian | Specification and loader | High for format; no real file tested | Older, future, and big-endian files are unsupported |
| Accept metadata types 0–12 | `gguf.h` enum | High | Unknown codes are unsupported |
| Enforce bounded resources | Project safety decision | Not an upstream guarantee | A valid but larger file can be classified unsupported |
| Follow spec multiple-of-8 alignment | Specification | High | Differs from loader's power-of-two check |
| Do not enforce key grammar beyond UTF-8 | Specification plus local choice | Documented relaxation | A malformed key can be accepted if it is safe and unknown |
| Preserve unknown tensor type IDs | `ggml.h` evolves | High | Catalog does not falsely name or reject the type |
| Summarise tokenizer arrays only | Privacy and scope decision | High | No vocabulary or merge content is emitted |
| Optional streaming SHA-256 | Project requirement | Implementation-tested | Full-file read only when explicitly requested |
| No cache, list command, grouping, or real-model cross-check | Superseded Stage 02 scope | Maintainer decision | Deferred; not silently claimed as delivered |

## 9. Conformance boundaries and non-claims

- GhostRAM does not claim to be a general GGUF validator or a llama.cpp-compatible
  loader. It accepts a bounded catalog subset.
- It rejects big-endian v2/v3 even though upstream v3 supports big-endian data.
- It does not enforce the specification's complete metadata key grammar.
- It does not verify contiguous tensor ranges, payload lengths, quantization block
  sizes, overlaps, or tensor contents.
- It does not validate that every architecture-specific metadata key required by
  the specification is present. Missing architecture metadata yields
  `metadata_state: incomplete`; other missing descriptive keys are simply omitted.
- It does not cross-check any file against an engine. No engine was installed.
- It does not claim that a real GGUF model parses. No real model was available or
  downloaded.
- It does not benchmark parsing, hashing, disk throughput, or memory use. No
  performance numbers are reported.
- The upstream sources are mutable `master` files, not a pinned release. Source
  observations are dated 2026-10-21 and require re-checking.

## 10. Follow-up evidence needed

A later authorized stage with an approved real model should cross-check catalog
metadata against the model's repeated header values and an engine-reported model
identity. That work must record the exact model hash, engine build, commands, and
observed output. It is not part of Stage 02 and must not be inferred from the
synthetic fixtures.
