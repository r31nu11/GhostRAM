# Project status

**Snapshot date:** 2026-10-20.
**Current stage:** Stage 01 — Minimal Rust workspace and diagnostic spine.
**Stage state:** Implementation and documentation complete; **pending formal
acceptance**.
**Branch:** `dev`.
**Commit state:** local history. The Stage 01 content commit is
`ed96c19ea3a84f8c7e981a61f8c13764695eb13f` and the report/status commit is
`02ad8bb8a8360b0021f849f195be8bf26ef43365`. The subprocess-safety follow-up commit
is the current HEAD; its exact hash is visible with `git log -1`. Nothing has been
pushed.
**Working tree:** clean after the safety-follow-up commit.
**Upstream:** `origin/dev` and `origin/main` remain at the starting commit
`bdfced9`.

## Summary

Stage 00 was accepted on 2026-10-20 and ADR-0001 through ADR-0005 were adopted at
the same time. Stage 01 then created a three-crate Rust workspace and a read-only
diagnostic path:

- `ghostram-core` contains typed byte quantities, diagnostic states, system
  models, typed errors, the `ghostram.doctor/1` envelope, and schema validation.
- `ghostram-probe` reads OS/kernel, CPU, RAM, swap/zram, storage, NVIDIA
  GPU/driver, Vulkan, tools, and llama.cpp discovery data without modifying the
  host.
- `ghostram-cli` provides `ghostram doctor`, `ghostram doctor --json`, help,
  version, and meaningful exit codes.

The command was run on the primary laptop. No engine or model was installed or
downloaded; no inference, benchmark, tuning, system change, push, tag, or release
was performed. Stage 01 is not accepted until the maintainer reviews it.

## Completed

- Stage 00 acceptance recorded, with ADR-0001 through ADR-0005 accepted on
  2026-10-20.
- Rust workspace with exactly `ghostram-core`, `ghostram-probe`, and
  `ghostram-cli`, plus a committed `Cargo.lock`.
- Exact `Bytes` quantities, diagnostic state/category models, system data models,
  typed errors with stable codes, and the versioned diagnostic envelope.
- Read-only probes for OS/kernel, CPU, memory, swap, zram, `/` storage, NVIDIA
  GPU/driver, Vulkan adapters, tool availability, and external llama.cpp
  executables.
- Bounded subprocess execution with explicit paths and argument arrays, output
  limits, isolated Linux process groups, group termination and direct-child reaping
  on timeout, and bounded output draining. No shell interpolation or inference
  launch.
- `ghostram doctor` human output and clean JSON output, including partial and
  unavailable categories when optional hardware or tools are absent.
- Parser fixtures and unit tests for successful, incomplete, malformed, and
  partial inputs; CLI integration tests.
- Rust formatting, Clippy, test, and locked-build jobs added to CI alongside the
  documentation and commit-message jobs.
- Versioned diagnostic schema, source-development instructions, and Stage 01
  report.
- Documentation updated for the implemented diagnostic path, accepted decisions,
  limitations, and remaining planned work.

## Project decisions (2026-10-20)

- ADR-0001 through ADR-0005 are Accepted. A changed decision requires a
  superseding ADR, not a silent rewrite.
- Engine build identity is immutable and path-independent. Installation records
  and timestamped capability observations are separate records tied to the build.
- No engine version/binary is pinned yet. Select the first baseline during Stage
  04 integration after build, checksum, dependency, device, and launch checks.
- No engine or model is installed in Stage 01. The first installation and smoke
  test belong to Stage 04 or later with separate maintainer approval.
- The MVP detects externally installed engines; do not bundle engine binaries. A
  later voluntary download of a verified build into a managed directory may be
  considered.
- No universal speed threshold. Define usability criteria by hardware and task
  profile after real measurements.
- Test the GTX 1070 after the laptop diagnostic/execution chain works, before
  serious performance claims and the first stable release.
- `dev` is the development and integration branch; `main` holds
  maintainer-accepted working/stable checkpoints. Push, tag, and GitHub Release
  remain separate maintainer decisions.

## Not completed / deferred

| Item | Reason | Target |
| --- | --- | --- |
| Formal Stage 01 acceptance | Maintainer review and acceptance are pending | Stage 01 gate |
| GGUF metadata parser | Requires dependency evaluation and parser tests | Stage 02 |
| Memory estimator | Needs metadata and calibration data | Stage 03 |
| Engine adapter and process supervisor | Requires adapter contract and fake-engine tests | Stage 04 |
| Engine installation/verification | Requires separate maintainer approval | Stage 04+ |
| Benchmark execution | No engine/model installed; no hardware tests approved | Stage 05 |
| Tuner and history store | Depends on measurement pipeline | Stage 06 |
| Iced UI | Depends on stable core services | Stage 07 |
| Packaging and installer | Depends on a release-candidate decision | Stage 08 |
| Windows support | Linux-first scope | Later |
| Secondary GTX 1070 validation | Machine not accessed | Later; schedule not set |

## Environment snapshot

These are observations from the Stage 01 diagnostic run, not guarantees that the
values remain unchanged.

| Item | Observed value |
| --- | --- |
| OS | CachyOS (rolling), kernel 7.2.11-1-cachyos, glibc 2.44 from Stage 00 |
| CPU | Intel Core i7-8750H, 6C/12T, AVX2/FMA/F16C, governor `powersave` |
| RAM | 7,989,465,088 bytes total; 3,470,852,096 bytes available at the report run |
| Swap | 7,989,096,448 bytes zram (`zstd`), 56,410,112 bytes used; no disk swap |
| GPU | NVIDIA GTX 1050 Ti Mobile, 4,294,967,296 bytes VRAM, compute capability 6.1 |
| GPU driver | 580.178.04; `nvidia-smi` reports maximum CUDA runtime 13.0 |
| Display | Intel UHD 630, KDE Wayland; NVIDIA GPU idle during inspection |
| Vulkan | Loader 1.4.363; NVIDIA device Vulkan 1.4.312, Intel Mesa 26.2.4 |
| Storage | 40.8 GiB free of 116.3 GiB on `/` at the report run |
| Rust | rustc/cargo 1.94.1, stable x86_64 |
| Missing tools | `cmake`, `ninja`, `nvcc`, CUDA toolkit, llama.cpp, Ollama, `gh` |
| Models | None found in the inspected common directories |

## Validation summary

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | Passed: 69 tests (13 core, 44 probe, 5 CLI unit, 7 CLI integration) |
| `cargo build --workspace --locked` | Passed |
| `ghostram doctor` | Completed on the primary laptop; overall `available` |
| `ghostram doctor --json` | Valid JSON; stdout contained only the envelope |
| `python3 scripts/check_docs.py` | Passed: 29 Markdown files, 122 relative links, 24 required files |
| `bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD` | Passed for all non-merge commits in the checked range; rerun after the final commit and recorded in the hand-off |
| Commit-message fixtures | Passed: valid message accepted; invalid header rejected with exit 1 |
| YAML parse of `.github/workflows/quality.yml` | Passed: jobs `commit-messages`, `documentation`, and `rust` |
| `git diff --check` / cached check | Clean |
| Shell syntax (`bash -n`) | Passed for both scripts and the hook |
| Local Git identity | `r31nu11` / `r31nu11@mail.ru`; global config unchanged |
| Commit authorship | Content commit verified as `r31nu11 <r31nu11@mail.ru>` for author and committer; report commit uses the same local identity |
| Remote state | `origin/dev` and `origin/main` still at `bdfced9`; no push |
| Hardware inference/benchmark | Not performed: no engine/model; no approval requested |

## Known limitations and blockers

- No GhostRAM release artifact, installer, or package exists. The diagnostic CLI is
  source-buildable only.
- Stage 01 probes Linux directly and has been exercised only on this x86_64 CachyOS
  laptop. Other distributions, kernels, NVIDIA driver versions, and machines are
  unverified.
- The `doctor` command inspects only external `PATH` entries for three llama.cpp
  executable names. It does not search arbitrary directories, verify a build hash,
  or prove backend, model, or inference support.
- Storage reporting covers `/` only. Additional model volumes are not measured.
- On Linux, timeouts kill the isolated process group and reap the direct child,
  and output pipes are drained with a bounded grace. A descendant that deliberately
  leaves the process group can survive; the bounded drain prevents it from blocking
  return but does not contain it.
- The `tools` category can report `available` when a found tool lacks version text.
  That means the executable was found; it does not prove a working toolchain.
- The workspace declares `rust-version = "1.85"`, but only Rust 1.94.1 was used.
  The declared MSRV and locked dependencies were not tested on 1.85.
- No engine build has been verified on this laptop by the project.
- CUDA 13.x cannot build new Pascal device code; a CUDA 12.x toolchain is required
  for a Pascal-targeted CUDA build, with Vulkan as fallback. A CUDA 13.x-labeled
  prebuilt remains artifact-specific and unverified.
- The CUDA runtime libraries (`libcudart`, cuBLAS) are not installed locally;
  official CUDA binaries may require a separate runtime asset or a toolkit.
- The 4 GiB VRAM and 7.4 GiB RAM budget is tight; usable model sizes are unknown
  until measured.
- The secondary machine is not available.
- No benchmark result exists, and no performance claim is supported.

## Next stage

Recommended: **Stage 02 — GGUF model catalog and identity.** It remains blocked
until Stage 01 is formally accepted and Stage 02 is explicitly authorized.

Stage 02 may discover and parse model metadata read-only, but it must not download
models or launch inference. Engine installation and the first smoke test remain in
Stage 04 or later with separate maintainer approval.

## Update rules

Update this file at the end of every stage. Record the actual branch, commit,
validation results, blockers, and next stage. Do not mark a stage accepted before
the maintainer has completed formal review and acceptance. Do not use this file
as a substitute for the detailed stage report.
