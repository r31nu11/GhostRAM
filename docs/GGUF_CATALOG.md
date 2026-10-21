# GGUF catalog behavior

**Status:** Implemented in Stage 02; source-only and pending formal acceptance.
There is no release artifact or installer.
**Implementation:** `crates/ghostram-models`.
**JSON contract:** [MODELS_SCHEMA.md](MODELS_SCHEMA.md).
**Format evidence:** [GGUF format research, 2026-10-21](research/2026-10-21-gguf-catalog-format.md).

The model catalog is a read-only inspection path for untrusted GGUF files. It
extracts compact descriptor and metadata summaries so a later planner can reason
about a locally selected model. It is not a loader, validator of inference
behavior, model downloader, cache, database, or engine integration.

## 1. Scope and non-goals

Implemented:

- one explicitly selected directory per scan;
- non-recursive traversal by default, with optional recursive descent;
- regular `.gguf` files only, matched case-insensitively by suffix;
- bounded GGUF v2/v3 little-endian header, metadata, and tensor-descriptor
  parsing;
- compact general, architecture, tokenizer-summary, and tensor-type metadata;
- conservative filename hints;
- optional streaming SHA-256 over the complete file;
- human-readable and versioned JSON output;
- per-file outcomes and bounded traversal diagnostics.

Not implemented or claimed:

- loading, mmap, dequantization, or inference with tensor data;
- architecture compatibility or engine support;
- shard grouping or reconstruction of a split model;
- model-family inference from filenames;
- real-model/engine metadata cross-checking;
- a metadata cache, invalidation policy, database, or persistent catalog;
- `ghostram models list`, download, watch mode, network access, or write access
  to a model tree;
- race-free observation of a file being concurrently modified.

The earlier roadmap wording for Stage 02 mentioned a cache, `ghostram models
list`, shard grouping, and a real-model cross-check. Those expectations were
superseded by the maintainer-authorized Stage 02 scope. The reconciliation is
recorded in [ROADMAP.md](ROADMAP.md) and the
[Stage 02 report](../reports/stages/stage-02-gguf-model-catalog.md).

## 2. Supported GGUF boundary

The parser accepts:

| Element | Supported |
| --- | --- |
| Magic | ASCII `GGUF` |
| Version | 2 or 3 |
| Byte order | Little-endian only |
| Header counts | 64-bit tensor and metadata counts |
| Metadata value types | Documented codes 0 through 12: `uint8`, `int8`, `uint16`, `int16`, `uint32`, `int32`, `float32`, `bool`, `string`, `array`, `uint64`, `int64`, `float64` |
| Arrays | Homogeneous element type, 64-bit declared count, nested arrays within the depth limit |
| Tensor descriptors | 64-bit name length, name, dimension count, dimensions, type identifier, data offset |
| Alignment | `general.alignment` as `uint32`; default `32`; must be a nonzero multiple of 8 |

GGUF v1, future versions, and unknown metadata or array element type codes are
rejected as unsupported. A big-endian v2/v3 header is detected when the version
bytes reverse to a supported version and is reported as
`unsupported_endianness`. This project intentionally supports only little-endian
files even though upstream GGUF v3 describes big-endian storage; the format has no
reliable universal byte-order marker for every possible invalid header.

Unknown tensor type identifiers are retained as catalog facts with no name and an
`unknown_tensor_type` warning. An unknown tensor type does not make the GGUF
structure malformed. It also does not imply that an engine can load it.

## 3. Directory scan boundary

`ghostram models scan <path>` requires exactly one path. No default directory,
`$HOME`, mount point, environment variable, or configured model directory is
invented.

- The selected root must exist, be a real directory, be readable, and have a
  UTF-8 display path. A symbolic-link root, regular-file root, unreadable root, or
  non-UTF-8 root is a fatal scan error.
- Paths in output are absolute but not canonicalized. Symbolic links are not
  resolved.
- Without `--recursive`, only regular files directly in the selected root are
  considered. Subdirectories are not traversed.
- With `--recursive`, subdirectories are traversed to a maximum depth of 32 below
  the selected root. The scanner observes at most 100,000 directory entries per
  scan.
- Symbolic-link files and directories are never followed. On Unix, candidate
  files are opened with `O_NOFOLLOW | O_NONBLOCK`; a replaced candidate is
  reported as unreadable or changed, and a raced named pipe cannot block the open
  indefinitely.
- Names ending in `.gguf` are matched with an ASCII case-insensitive suffix
  comparison. Only regular files are cataloged.
- Non-UTF-8 candidate paths are counted, not cataloged, and make the result
  `partial`.
- Directory entries are sorted for deterministic output, and final file records
  are sorted lexicographically by path string.
- Recursive traversal is symlink-conservative but not a race-free snapshot. A
  directory can be replaced between type inspection and descent.
- At most 100 traversal issues are retained. Counts still reflect omitted issues,
  and `skipped.issues_truncated` is set.
- The command never writes to the selected tree, creates a cache or database,
  changes model files, or initiates network requests. A user-selected path may of
  course reside on a network filesystem.

A per-file open, read, parse, or hash failure is contained in a `partial` result.
A failure to read the selected root or produce output is a command failure.

## 4. Bounded parsing

The parser is an in-tree minimal GGUF reader, not a general-purpose model loader.
It reads the header, metadata, and tensor descriptors through a 64 KiB buffered
reader and skips unneeded encoded data with checked seeks. It does not allocate
from an untrusted element count.

| Limit | Value | Effect |
| --- | --- | --- |
| Metadata entries | 65,536 | More is `limit_exceeded`. |
| Metadata key bytes | 65,535 | Per key; more is `limit_exceeded`. |
| Total metadata key bytes | 8 MiB | Combined keys in one file. |
| Retained recognized string | 16 KiB | Larger values are skipped and warned about; this never retains a truncated string as a value. |
| Any encoded string | 64 MiB | Larger encoded lengths are rejected. A skipped string is not UTF-8 validated. |
| Array elements | 4,000,000 | Per array. |
| Total array elements | 8,000,000 | Combined across all arrays in one file. |
| Nested-array depth | 8 | Levels below the top-level array. |
| Tensor descriptors | 1,000,000 | Declared tensor count. |
| Tensor name | 64 bytes | Per encoded name. |
| Total tensor-name bytes | 32 MiB | Combined names in one file. |
| Tensor dimensions | 4 | Per descriptor. |
| Tensor dimension/product | `i64::MAX` | Each dimension and the product must fit this bound. |
| Distinct tensor types | 256 | Per file. |
| Hash buffer | 64 KiB | Fixed-size streaming buffer. |

These are conservative product limits, not claims that GGUF is defined with these
limits. A file that exceeds a product limit is classified `unsupported` because
the catalog refuses to process it, even if another reader might accept it.

The parser validates:

- the 24-byte minimum header, magic, version, and byte-order heuristic;
- all header counts before allocating or iterating;
- metadata key length, UTF-8 of keys, and duplicate keys;
- documented metadata type codes, including array element type codes;
- boolean scalar and array values (`0` or `1`);
- nested-array depth and combined element budgets;
- alignment metadata is `uint32`, nonzero, and a multiple of 8;
- tensor names are non-empty UTF-8, at most 64 bytes, unique, and within the total
  name budget;
- up to four dimensions, positive arithmetic and count products, and the
  dimension bound;
- tensor data offsets are multiples of the effective alignment and start within
  the available tensor-data region;
- truncation and filesystem read errors as typed failures.

The parser does not validate every semantic architecture requirement, tokenizer
consistency, quantization block payload sizes, tensor overlaps, or each tensor's
full byte range. Tensor starts are bounded, but payload lengths and contents are
not parsed. Extra trailing bytes are reported as `tensor_data_bytes` and are not
interpreted.

Metadata keys and retained recognized strings are validated as UTF-8. Strings
that are skipped without retention, including chat templates, tokenizer arrays,
and unknown string values, are bounds-checked and skipped but their UTF-8 content
is not validated. This is sufficient for a compact catalog and avoids reading or
retaining large tool-only values; it is not a full file-format validator.

## 5. Validation and classification

| Outcome | Meaning |
| --- | --- |
| `parsed` | Header, metadata, and descriptors were structurally accepted. Warnings and `metadata_state: incomplete` are still possible. |
| `unsupported` | Version, endianness, metadata type, or a product limit is outside this build's boundary. |
| `malformed` | Bad magic, invalid alignment, duplicate key/name, invalid tensor, truncation, or another structural invariant failed. |
| `unreadable` | The candidate could not be opened, read, or observed as a regular file. |
| `changed` | Identity, size, or modification time changed during inspection, or the hash read length differed from the initial size. Any parsed metadata and digest are discarded. |

Recognized keys with unexpected types generally produce
`unexpected_metadata_type` warnings and are omitted rather than guessed.
`general.alignment` is an exception: any present value not encoded as `uint32` is
`invalid_alignment` and rejects the file. Missing architecture metadata yields
`metadata_state: incomplete` and a warning, not a malformed file.

Scan `state` is `partial` if any candidate is non-`parsed`, any traversal issue is
retained, an entry or depth limit affected the result, an unreadable directory or
entry occurred, a non-UTF-8 candidate path was skipped, or issue details were
truncated. A completed `partial` or `empty` scan still exits 0.

## 6. Metadata extraction

The catalog retains only compact facts:

- GGUF version and declared metadata/tensor counts;
- effective alignment and tensor-data offset/available byte count;
- `general.architecture`, `name`, `basename`, `finetune`, `size_label`,
  `quantization_version`, `file_type`, and explicit `parameter_count`;
- common per-architecture context/embedding/block/feed-forward/attention/expert/
  RoPE counts;
- tokenizer model and array summaries for tokens, scores, token types, merges,
  and added tokens;
- booleans for chat-template and Hugging Face tokenizer JSON presence;
- tensor types and descriptor counts;
- bounded warnings.

It never emits token vocabulary entries, scores, merge pairs, added-token values,
chat-template text, Hugging Face tokenizer JSON, prompts, tensor bytes,
embeddings, or arbitrary metadata blobs. File-type labels and tensor-type names
come from a reviewed local mapping, not from arbitrary metadata text.

Quantization is a hint. It is the recognized `general.file_type` label when
available, otherwise the most common reviewed tensor type. A recognized label is
marked `explicit`; a tensor-type fallback is marked `derived`. Unknown file types
and unknown tensor types do not prove an unsupported or supported engine
configuration.

Filename hints recognize `mmproj-`, `mtp-`, `-lora`, and `-vocab` naming and a
`NNNNN-of-MMMMM` shard suffix. They are informational only. Shards remain separate
records and are never merged.

## 7. Optional SHA-256

SHA-256 is never computed implicitly. `--hash` streams each readable candidate
from start to EOF through SHA-256 with a fixed 64 KiB buffer. This reads the full
file, including tensor bytes that metadata parsing did not touch.

- A digest is reported only after a successful read to EOF.
- A read error produces `failed` and no digest.
- A discarded digest produces `changed` and no digest.
- If file identity, observed size, or modification time changes, or the number of
  bytes hashed differs from the starting size, the digest is discarded.
- Change detection is best effort. The scanner holds no lock, does not claim a
  consistent point-in-time snapshot, and can miss a same-size, same-metadata
  concurrent modification. It never reports a partial digest as complete.

The hash covers malformed and unsupported candidates too, because hashing is an
independent byte-level observation. Hashing a very large file takes proportionally
longer; no performance or throughput claim is made.

## 8. Command examples

From a source checkout:

```sh
cargo run -p ghostram-cli -- models scan ./models
cargo run -p ghostram-cli -- models scan ./models --recursive
cargo run -p ghostram-cli -- models scan ./models --recursive --hash
cargo run -p ghostram-cli -- models scan ./models --json
```

The command is source-development functionality, not an installed release. The
selected path is the only path inspected. Use `--` before a path that begins with
a hyphen.

Exit codes:

- `0`: scan completed, including `partial` and `empty`;
- `1`: scan setup, root, serialization, or stdout failure;
- `2`: invalid command invocation.

## 9. Privacy and safety

The catalog reads local files and emits absolute paths, sizes, modification times,
and optional hashes. Those fields can reveal directory layout. It does not emit
model contents, tokenizer values, chat templates, prompts, or arbitrary metadata
blobs. Review JSON before sharing it.

The scanner performs no writes, no cache updates, no engine launches, no model
downloads, no package installation, and no network operation of its own. It does
not require root and does not alter permissions, drivers, clocks, or system
settings. The optional hash consumes read bandwidth and CPU proportional to file
size; there is no hardware benchmark or inference.

## 10. Known limitations and non-claims

- No real GGUF model was available in the development environment, so the parser
  and scanner were validated with deterministic synthetic and malformed fixtures,
  not a real model.
- No engine was installed, so no metadata was cross-checked against engine-reported
  model properties.
- Big-endian GGUF is unsupported. The detection heuristic covers big-endian v2/v3
  version headers; other non-little-endian structures fail as unsupported type or
  version/endian errors.
- Unknown skipped strings are not UTF-8 validated, and tensor payload contents and
  full ranges are not validated.
- Tensor offsets are checked as starts within the data region, not as complete
  non-overlapping payload ranges.
- A scan is not a cache and cannot answer "what changed since the last scan"
  without the caller comparing records.
- Recursive traversal and change/hash consistency are not race-free against a
  concurrent writer.
- There is no I/O timeout for a path that blocks indefinitely. Only regular files
  are cataloged; a pathological filesystem can still block a read or seek.
- No result is a claim that an inference engine supports an architecture,
  quantization, tokenizer, or context length.

Upstream format sources and the reasoning behind these boundaries are recorded in
[the GGUF research note](research/2026-10-21-gguf-catalog-format.md). The JSON field
contract is in [MODELS_SCHEMA.md](MODELS_SCHEMA.md).
