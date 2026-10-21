# Project status

**Snapshot date:** 2026-10-21.
**Current stage:** Stage 02 — GGUF model catalog and identity.
**Stage state:** Implementation and documentation complete; **pending formal
acceptance**.
**Branch:** `dev`.
**Commit state:** Stage 02 content commit
`244809bf12fda854f22d018c36cab5bd1d253b40`
(`feat(stage-02): add bounded GGUF model catalog`). The final report/status commit
is the Stage 02 HEAD; use `git log -1` for its exact hash. Both commits use the local
maintainer identity `r31nu11 <r31nu11@mail.ru>`.
**Publication:** the maintainer explicitly authorized publication of the Stage
00/01/02 development history to `origin/dev`. This is not authorization for a tag,
GitHub Release, artifact, or promotion to `main`. At content freeze, `origin/dev`
and `origin/main` were both at `bdfced9`.
**Working tree:** clean immediately before the final report/status commit, which
contains only the final Stage 02 status and report updates.

## Summary

Stage 00 was accepted on 2026-10-20 and ADR-0001 through ADR-0005 were adopted at
the same time. Stage 01 was accepted on 2026-10-21. Stage 02 adds the fourth
workspace crate and a bounded, read-only model inspection path:

- `ghostram-models` parses GGUF v2/v3 little-endian headers, metadata, and tensor
  descriptors with checked reads and explicit limits. It does not parse tensor
  payloads.
- Compact extraction retains general identity fields, common architecture counts,
  tokenizer array summaries and presence flags, tensor type descriptors, alignment,
  and bounded warnings. It never retains token vocabulary values, scores, merge
  pairs, added-token values, chat templates, Hugging Face tokenizer JSON, prompts,
  or arbitrary blobs.
- Directory scanning requires one explicitly selected root. It is non-recursive by
  default, supports `--recursive`, never follows symlinks, skips non-regular and
  non-UTF-8-path candidates with diagnostics, and sorts records deterministically.
- Optional `--hash` streams complete files through SHA-256 with a fixed 64 KiB
  buffer. Without it, tensor payloads are not read. A digest is reported only after
  a complete successful read and is discarded when a change is observed.
- `ghostram models scan` provides human and `--json` output under the versioned
  `ghostram.models.scan/1` schema. `ghostram.doctor/1` is unchanged.
- Deterministic synthetic and adversarial fixtures cover all documented metadata
  types, nested arrays, tokenizer summaries, tensor types, malformed and truncated
  files, resource limits, symlinks, hashing, change detection, traversal, and CLI
  behavior.

**No real GGUF model was available or tested.** All parser and scanner evidence is
from synthetic fixtures. No engine is installed, so model metadata was not
cross-checked against engine-reported properties. No inference, GPU compute,
model download, network test, benchmark, or hardware measurement was performed in
Stage 02.

## Current capabilities

| Capability | State |
| --- | --- |
| `ghostram doctor` / `doctor --json` | Implemented read-only diagnostics; source-only |
| Linux hardware, GPU, Vulkan, tool, and engine probes | Implemented read-only; no engine installed |
| GGUF v2/v3 little-endian metadata parser | Implemented with explicit limits; synthetic fixtures only |
| `ghostram models scan` / `--json` | Implemented; one user-selected directory; source-only |
| Recursive scan | Implemented as opt-in `--recursive`; depth and entry limits apply |
| SHA-256 | Optional explicit `--hash`; streaming and complete-read required |
| Stable model identity | **Not implemented**; scan records are observations |
| Shard grouping, `models list`, cache, database | **Not implemented**; earlier scope superseded |
| Real-model/engine cross-check | **Not implemented**; no model or engine available |
| Model loading or inference | **Not implemented** |
| Planner, tuner, store, GUI, benchmarks | **Not implemented** |

## Stage 02 decisions

- No new ADR was required. Adding `ghostram-models` follows the crate-split rule in
  ADR-0002 and does not rewrite an accepted decision.
- The parser boundary is GGUF v2/v3 little-endian. Big-endian v2/v3 headers are
  detected and classified `unsupported_endianness`; v1 and future versions are
  unsupported.
- Unknown metadata type codes are rejected. Unknown tensor type identifiers are
  retained without an invented name and produce `unknown_tensor_type`; they do not
  make a structurally valid file malformed.
- `general.alignment` must be `uint32`, nonzero, and a multiple of 8. Any present
  non-`uint32` encoding is a fatal `invalid_alignment`, not a silent default.
- `tokenizer.ggml.added_tokens` is treated as `array[string]`, matching upstream.
- The parser follows the GGUF specification's 64-byte tensor-name maximum and
  multiple-of-8 alignment rule. The upstream loader's stricter power-of-two
  alignment and its effective `<64` tensor-name boundary are documented as
  deviations, not silently adopted.
- Keys are checked for length, UTF-8, emptiness, and duplicates, but the full
  ASCII lower-snake key grammar is not enforced. This relaxed behavior is
  documented in the format research note.
- An observed file change discards both any parsed metadata and any digest, and
  records `parse_status: changed` with best-effort detection.
- The earlier roadmap scope for a cache, `models list`, shard grouping, configured
  default directories, mandatory hashing, and a real-model cross-check was
  superseded by maintainer authorization. Those items are not hidden acceptance
  criteria and must not be reported as delivered.

## Validation snapshot

| Check | Observed result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed (no output) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | 147 passed, 0 failed (12 CLI unit, 15 CLI integration, 16 core, 63 models, 41 probe) |
| `cargo build --workspace --locked` | Passed |
| `python3 scripts/check_docs.py` | Passed: 33 Markdown files, 163 relative links, 28 required files |
| `bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD` | Passed over `bdfced9..244809b`; rerun over the completed history in the final hand-off |
| Local author identity | `r31nu11` / `r31nu11@mail.ru` |
| Complete diff inspection | Passed before staging |

Exact commands and output for the committed state are recorded in the
[Stage 02 report](../reports/stages/stage-02-gguf-model-catalog.md).

## Environment snapshot

Stage 02 did not take a new hardware measurement. The last accepted read-only
hardware snapshot is in the
[Stage 01 report](../reports/stages/stage-01-minimal-rust-workspace.md): the
primary laptop has a 4 GiB Pascal GPU and 7.4 GiB RAM; the secondary machine has
not been accessed. No engine was installed. No real model was supplied, downloaded,
or scanned, and no benchmark was run.

## Not done and deferred

- No real GGUF file or engine cross-check. This remains the largest validation gap;
  synthetic fixtures cannot prove compatibility with a real model family.
- No shard grouping or reconstruction, `models list`, persistent cache, database,
  watch mode, or comparison between scans.
- No stable `ModelId`: optional hashes and paths are observations, and another
  stage must define when a hash is mandatory for identity.
- No big-endian or GGUF v1 support.
- No tensor payload validation, quantization block-size checks, overlap checks, or
  inference-engine acceptance claim.
- No release artifact, installer, tag, GitHub Release, or `main` promotion.
- No Stage 03 work. Stage 03 remains blocked until Stage 02 is accepted.

## Next stage

Stage 03 (memory planner and safety budgets) is next but blocked on formal Stage 02
acceptance. It must not start from this status alone. The maintainer must review the
Stage 02 report, all deferred scope, and the validation evidence.
