# Model scan schema

**Status:** Implemented in Stage 02; source-only and pending formal acceptance.
There is no release artifact or installer.
**Schema identifier:** `ghostram.models.scan/1`.
**Implementation:** `crates/ghostram-models/src/catalog.rs` and
`crates/ghostram-models/src/scan.rs`.

`ghostram models scan` is a read-only, bounded GGUF metadata scan. It inspects one
explicitly selected directory, records observed file and metadata facts, and
reports files it could not catalog. It does not load tensors, launch an engine,
download a model, write a cache, or contact a network service.

The JSON output is a versioned interchange contract, not a model-identity claim.
Paths and filename hints are observations; a path alone is not a stable model ID.

## 1. Envelope

`ghostram models scan --json` writes one JSON object followed by a newline to
stdout. Human-readable text is not mixed into stdout. Operational errors go to
stderr with a non-zero exit code.

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | Always `ghostram.models.scan/1` for this build. |
| `application_version` | string | Version of the `ghostram` binary that generated the scan. Source builds currently report `0.1.0-dev`. |
| `generated_at` | string | UTC RFC 3339 generation time with whole-second precision, for example `2026-10-21T12:00:00Z`. |
| `scan` | object | Scan payload described below. |

Consumers must reject an unknown major schema identifier rather than guessing at
field meanings. Unknown additional fields in a known schema should be ignored.

## 2. Scan payload

| Field | Type | Meaning |
| --- | --- | --- |
| `state` | string | `complete`, `partial`, or `empty`. |
| `roots` | array of strings | Exactly one absolute, non-canonicalized path selected by the user. |
| `options` | object | `recursive` and `hash` booleans that were actually used. |
| `summary` | object | Aggregate counts for the scan. |
| `skipped` | object | Entries intentionally not cataloged, by reason. |
| `issues` | array of objects | Bounded traversal issue details. |
| `files` | array of objects | Candidate files, sorted lexicographically by path string. |

### 2.1 Scan states

| State | Meaning |
| --- | --- |
| `complete` | Traversal completed, no retained traversal issue occurred, and every candidate has `parse_status: "parsed"`. Parsed files may still have warnings or incomplete descriptive metadata. |
| `partial` | At least one candidate did not parse, at least one traversal entry could not be inspected, a limit stopped traversal, a retained issue exists, or a non-UTF-8 candidate path was skipped. |
| `empty` | Traversal completed with no candidate `.gguf` files and no condition that would make the scan partial. |

A `partial` result is a successful scan result. It is not a command failure, and
it does not mean the valid files in the result are untrustworthy. It means some
part of the selected tree or candidate set was not fully inspected.

### 2.2 Summary

Byte counts are exact JSON integers, not rounded strings.

| Field | Meaning |
| --- | --- |
| `candidates` | Candidate `.gguf` files inspected. |
| `parsed` | Candidates with `parse_status: "parsed"`. |
| `parsed_with_warnings` | Parsed candidates with at least one warning. |
| `incomplete_metadata` | Parsed candidates whose descriptive metadata state is `incomplete`. |
| `unsupported` | Candidates classified `unsupported`. |
| `malformed` | Candidates classified `malformed`. |
| `unreadable` | Candidates classified `unreadable`. |
| `changed` | Candidates that changed during inspection. |
| `hash_computed` | Files with a complete SHA-256 digest. |
| `hash_failed` | Files where hashing was requested and failed. |
| `hash_changed` | Files where a digest was discarded because the file changed. |
| `bytes_hashed` | Total bytes streamed for SHA-256. Zero when hashing was not requested. |

The per-file categories and `candidates` use the same records, so their counts are
consistent. A candidate may contribute to both a parse-status count and a hash
count.

### 2.3 Skipped entries

| Field | Meaning |
| --- | --- |
| `symlinks` | File or directory symbolic links encountered. Symlinks are never followed. |
| `non_regular_files` | Non-directory, non-symlink entries whose names end in `.gguf` but which are not regular files (for example devices or sockets). Symbolic links are counted separately. |
| `non_gguf_files` | Regular files whose names do not end in `.gguf` (ASCII case-insensitive). |
| `non_utf8_paths` | Candidate paths that could not be represented as UTF-8 and were not cataloged. |
| `max_depth_directories` | Directories not descended into because the depth limit was reached. |
| `entry_limit_reached` | Whether the total directory-entry limit stopped traversal. |
| `unreadable_directories` | Directories whose entries could not be read during recursive traversal. A failure on the selected root is fatal instead. |
| `unreadable_entries` | Directory entries whose type or metadata could not be read. |
| `issues_truncated` | Whether more issue details existed than the bounded issue list retained. |

Ordinary non-GGUF files and symlinks do not by themselves make the scan
`partial`. Limit and unreadability conditions do.

### 2.4 Traversal issues

Each `issues` entry has:

| Field | Meaning |
| --- | --- |
| `path` | Affected path as a display string, when available. |
| `kind` | `unreadable_directory`, `unreadable_entry`, `entry_limit_reached`, or `depth_limit_reached`. |
| `message` | Human-readable detail. |

At most 100 issue records are retained. `skipped.issues_truncated` records whether
additional issues were observed.

## 3. File entries

| Field | Type | Presence | Meaning |
| --- | --- | --- | --- |
| `path` | string | always | Absolute, non-canonicalized candidate path. |
| `file_size` | integer | optional | File size observed before parsing. Omitted only when an unreadable candidate had no observable regular-file metadata. Parsed files always have it. |
| `modified_at` | string | optional | UTC RFC 3339 modification time observed before parsing. |
| `parse_status` | string | always | `parsed`, `unsupported`, `malformed`, `unreadable`, or `changed`. |
| `failure` | object | optional | Stable failure category, message, and optional byte offset. Present for non-parsed or changed files. |
| `metadata` | object | optional | Compact metadata for a parsed file. |
| `hash` | object | always | SHA-256 outcome, including when not requested. |
| `filename_hints` | object | always | Filename-only hints. Empty object when none apply. |

### 3.1 Parse statuses

| Status | Meaning |
| --- | --- |
| `parsed` | The GGUF structure was accepted. `metadata` is present. |
| `unsupported` | The version, endianness, metadata shape, or a resource limit is outside this build's supported boundary. |
| `malformed` | Magic, alignment, duplicate names, tensor descriptors, truncation, or another structural invariant failed. |
| `unreadable` | The candidate could not be opened or read as a regular file. |
| `changed` | File identity, size, or modification time changed during inspection. Any parsed result and digest are discarded. |

A `changed` result takes precedence over a parser failure. Its `failure` is the
original parser failure when one existed, otherwise a `changed` failure.

### 3.2 Failure categories

| `failure.kind` | Typical classification |
| --- | --- |
| `invalid_magic` | malformed |
| `unsupported_version` | unsupported |
| `unsupported_endianness` | unsupported |
| `unsupported_metadata_type` | unsupported |
| `invalid_alignment` | malformed |
| `duplicate_metadata_key` | malformed |
| `duplicate_tensor_name` | malformed |
| `invalid_tensor` | malformed |
| `truncated` | malformed |
| `malformed` | malformed |
| `limit_exceeded` | unsupported |
| `io` | unreadable |
| `changed` | changed |

`failure.message` is bounded diagnostic text. `failure.offset` is the byte offset
associated with the failure when the parser knows it. Messages and offsets are
observations, not a stable ABI.

## 4. Metadata object

`metadata` is present only for `parse_status: "parsed"`. It never contains token
strings, merge tables, chat-template text, prompts, tensor payloads, embeddings,
or arbitrary metadata blobs.

| Field | Meaning |
| --- | --- |
| `gguf_version` | Parsed GGUF version, `2` or `3`. |
| `tensor_count` | Tensor descriptor count declared by the file. |
| `metadata_entry_count` | Metadata key/value count declared by the file. |
| `metadata_state` | `available` when a non-empty `general.architecture` was retained; otherwise `incomplete`. |
| `alignment` | Effective alignment as `{ "value": integer, "source": provenance }`. |
| `tensor_data_offset` | Byte offset of the aligned tensor-data region. |
| `tensor_data_bytes` | Bytes available after that offset. Payloads were not parsed or validated. |
| `general` | General model fields described below. |
| `architecture_fields` | Architecture-specific numeric fields described below. |
| `tokenizer` | Tokenizer summaries and presence flags. |
| `tensor_types` | Tensor type identifiers and descriptor counts, sorted by identifier. |
| `warnings` | Non-fatal warnings in deterministic parse order. |

### 4.1 Provenance

Fields that carry an interpreted value use `{ "value": ..., "source": ... }`.

| Source | Meaning |
| --- | --- |
| `explicit` | Stored directly in accepted GGUF metadata. |
| `derived` | Derived from accepted structural data, such as tensor descriptors. |
| `default` | GGUF specification default used because the key was absent. |

`alignment.source` is `explicit` when `general.alignment` was accepted; otherwise
it is `default` with value `32`. `general.quantization.source` is `explicit` when
derived from a reviewed `general.file_type` label and `derived` when inferred from
the most common reviewed tensor type.

### 4.2 General fields

All fields are optional and are omitted when unavailable.

| Field | Value type | Rule |
| --- | --- | --- |
| `architecture` | string | `general.architecture`; empty values are treated as absent. |
| `name` | string | `general.name`. |
| `basename` | string | `general.basename`. |
| `finetune` | string | `general.finetune`. |
| `size_label` | string | `general.size_label`. |
| `quantization_version` | integer | `general.quantization_version`, retained when it fits `uint32`. |
| `file_type` | object | `general.file_type` as `{ "code": integer, "label": string? }`; the label is present only for reviewed codes. |
| `parameter_count` | integer | `general.parameter_count` when explicitly stored. |
| `quantization` | string | Reviewed file-type label without `MOSTLY_`/`ALL_`, or a fallback from the most common reviewed tensor type. |

Reviewed file-type labels are:

| Code | Label | Code | Label | Code | Label |
| --- | --- | --- | --- | --- | --- |
| 0 | `F32` | 7 | `Q8_0` | 14 | `Q4_K_S` |
| 1 | `F16` | 8 | `Q5_0` | 15 | `Q4_K_M` |
| 2 | `Q4_0` | 9 | `Q5_1` | 16 | `Q5_K_S` |
| 3 | `Q4_1` | 10 | `Q2_K` | 17 | `Q5_K_M` |
| 4 | `Q4_1_SOME_F16` | 11 | `Q3_K_S` | 18 | `Q6_K` |
| 5 | `Q4_2` | 12 | `Q3_K_M` |  |  |
| 6 | `Q4_3` | 13 | `Q3_K_L` |  |  |

The table is the compact quantization spelling emitted in `quantization`; the
underlying file-type label uses `ALL_F32` or `MOSTLY_<name>`. Unrecognized codes
retain `file_type.code` but omit `file_type.label`. There is no guess at a
label's meaning.

### 4.3 Architecture fields

These keys are read only when `general.architecture` was retained. Keys use the
`<architecture>.<suffix>` form and must be unsigned integer metadata. Each field
is optional and omitted when absent.

| Field | GGUF suffix | Unit |
| --- | --- | --- |
| `context_length` | `context_length` | tokens |
| `embedding_length` | `embedding_length` | values |
| `block_count` | `block_count` | blocks/layers |
| `feed_forward_length` | `feed_forward_length` | values |
| `attention_head_count` | `attention.head_count` | heads |
| `attention_head_count_kv` | `attention.head_count_kv` | heads |
| `attention_key_length` | `attention.key_length` | values |
| `attention_value_length` | `attention.value_length` | values |
| `expert_count` | `expert_count` | experts |
| `expert_used_count` | `expert_used_count` | experts per token |
| `rope_dimension_count` | `rope.dimension_count` | dimensions |

This contract recognizes common architecture fields. It does not validate that a
particular architecture uses every field, nor does it enumerate every
architecture-specific key.

### 4.4 Tokenizer object

| Field | Meaning |
| --- | --- |
| `model` | `tokenizer.ggml.model` string. |
| `tokens` | `{ "element_type": ..., "count": ... }` for `tokenizer.ggml.tokens`. |
| `scores` | Array summary for `tokenizer.ggml.scores`. |
| `token_types` | Array summary for `tokenizer.ggml.token_type`. |
| `merges` | Array summary for `tokenizer.ggml.merges`. |
| `added_tokens` | Array summary for `tokenizer.ggml.added_tokens`; the upstream type is `array[string]`. |
| `chat_template_present` | Whether `tokenizer.chat_template` was present. The value is not retained. |
| `huggingface_json_present` | Whether `tokenizer.huggingface.json` was present. The value is not retained. |

An array summary contains only the encoded element type and declared element
count. The elements are skipped with the same bounded streaming rules as unknown
arrays. The contract never emits vocabulary entries, scores, merge pairs, added
tokens, chat-template text, or Hugging Face tokenizer JSON.

### 4.5 Tensor types

`tensor_types` contains one object per distinct encoded tensor type:

```json
{ "type_id": 12, "name": "Q4_K", "tensor_count": 8 }
```

`name` is omitted when this build has no reviewed name. Unknown type identifiers
do not invalidate an otherwise structurally valid file; they are retained with
`type_id` and a warning. This is catalog information, not a claim that an engine
can load the type.

### 4.6 Warnings

| `warnings[].code` | Meaning |
| --- | --- |
| `missing_architecture` | Required `general.architecture` was absent, empty, or unusable. |
| `unexpected_metadata_type` | A recognized key had an unexpected metadata type or value width. |
| `oversized_metadata_value` | A recognized string exceeded the retained-string limit and was omitted. |
| `unknown_tensor_type` | A tensor type had no reviewed name. |

Warnings do not make the file malformed, but they may make `metadata_state`
`incomplete`. Messages are bounded, human-readable details.

## 5. Hash object

`hash` is always present, including when `--hash` was not supplied.

| Field | Presence | Meaning |
| --- | --- | --- |
| `algorithm` | always | Currently `sha256`. |
| `status` | always | `not_requested`, `computed`, `failed`, or `changed`. |
| `sha256` | only for `computed` | Lowercase hexadecimal digest of a complete successful read. |
| `bytes_hashed` | always | Bytes streamed by the hash operation; zero when not requested. |
| `reason` | for `failed` or `changed` | Bounded explanation. |

With `--hash`, every readable candidate is hashed, including candidates whose
GGUF metadata is malformed or unsupported. Hashing reads the complete file,
including unparsed tensor bytes. It uses a fixed 64 KiB buffer and does not load
the file into memory.

A digest is emitted only after a successful read to EOF. A read error produces
`failed` with no digest. If the file's identity, size, or observed modification
time changes, or if the number of bytes read differs from the initial size, the
result is `changed` and the digest is discarded. Change detection is best effort;
the scanner does not hold a lock and does not claim a race-free snapshot against
an external writer.

## 6. Filename hints

`filename_hints` is a convenience only. It never changes the parsed metadata and
never asserts model identity.

| Field | Meaning |
| --- | --- |
| `role` | Optional `mmproj`, `mtp`, `lora`, or `vocab` from conservative filename prefixes/suffixes. |
| `shard` | Optional `{ "number": 1, "total": 2 }` parsed from a suffix such as `00001-of-00002`. |

The scanner does not group shards, reconstruct a complete model, or infer a model
family from a filename. A shard file remains an independent catalog entry.

## 7. Output, exit codes, and privacy

| Invocation | stdout | Exit code |
| --- | --- | --- |
| `ghostram models scan <path>` | Human-readable summary and file records | 0 when the scan completed, including `partial` and `empty` |
| `ghostram models scan <path> --json` | Exactly one JSON envelope and a trailing newline | 0 when the scan completed |
| Invalid option, missing path, or extra positional argument | Nothing | 2 |
| Root rejection, scan setup failure, serialization failure, or stdout failure | Nothing reliable | 1 |

A rejected root includes a symbolic-link root, a non-directory root, an
unreadable selected directory, or a root that is not valid UTF-8. Per-file
problems do not abort a completed scan.

The schema contains absolute file paths, modification times, and optionally
content hashes. Those can reveal local directory structure. It does not contain
token values, merge tables, chat templates, prompts, embeddings, tensor bytes, or
arbitrary metadata blobs. Review a JSON report before publishing it.

## 8. Example

This reduced example is illustrative, not a test fixture. Optional fields are
omitted where unavailable.

```json
{
  "schema_version": "ghostram.models.scan/1",
  "application_version": "0.1.0-dev",
  "generated_at": "2026-10-21T12:00:00Z",
  "scan": {
    "state": "complete",
    "roots": ["/models"],
    "options": {
      "recursive": false,
      "hash": false
    },
    "summary": {
      "candidates": 1,
      "parsed": 1,
      "parsed_with_warnings": 0,
      "incomplete_metadata": 0,
      "unsupported": 0,
      "malformed": 0,
      "unreadable": 0,
      "changed": 0,
      "hash_computed": 0,
      "hash_failed": 0,
      "hash_changed": 0,
      "bytes_hashed": 0
    },
    "skipped": {
      "symlinks": 0,
      "non_regular_files": 0,
      "non_gguf_files": 2,
      "non_utf8_paths": 0,
      "max_depth_directories": 0,
      "entry_limit_reached": false,
      "unreadable_directories": 0,
      "unreadable_entries": 0,
      "issues_truncated": false
    },
    "issues": [],
    "files": [
      {
        "path": "/models/example.gguf",
        "file_size": 1536,
        "modified_at": "2026-10-20T18:30:00Z",
        "parse_status": "parsed",
        "metadata": {
          "gguf_version": 3,
          "tensor_count": 0,
          "metadata_entry_count": 3,
          "metadata_state": "available",
          "alignment": {
            "value": 32,
            "source": "default"
          },
          "tensor_data_offset": 320,
          "tensor_data_bytes": 1216,
          "general": {
            "architecture": {
              "value": "llama",
              "source": "explicit"
            },
            "file_type": {
              "value": {
                "code": 15,
                "label": "MOSTLY_Q4_K_M"
              },
              "source": "explicit"
            },
            "quantization": {
              "value": "Q4_K_M",
              "source": "explicit"
            }
          },
          "architecture_fields": {},
          "tokenizer": {
            "chat_template_present": false,
            "huggingface_json_present": false
          },
          "tensor_types": [],
          "warnings": []
        },
        "hash": {
          "algorithm": "sha256",
          "status": "not_requested",
          "bytes_hashed": 0
        },
        "filename_hints": {}
      }
    ]
  }
}
```

## 9. Compatibility rules

- Additive optional fields may be added within `ghostram.models.scan/1` when
  existing field meanings do not change.
- Renaming, removing, changing a type, changing enum values, or changing the
  meaning of a field requires a new schema identifier such as
  `ghostram.models.scan/2`.
- Consumers should ignore unknown additional fields and reject an unknown schema
  identifier.
- A scan is a point-in-time local observation, not a model-load test, an engine
  compatibility result, or a benchmark.
- `ghostram.doctor/1` is a separate contract and is not changed by this schema.
