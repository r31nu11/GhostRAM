# Project status

**Snapshot date:** 2026-10-20.
**Current stage:** Stage 00 — Technical Reconnaissance and Project Charter.
**Stage state:** Documentation and repository foundation complete; **awaiting
human review and acceptance**.
**Current branch:** `dev`.
**Stage 00 content commit:** recorded in the Stage 00 report and in Git history.
**Working tree:** clean after the Stage 00 status commit.

## Summary

Stage 00 established the project contract, architecture proposal, engine research,
benchmark methodology, repository workflow, and quality checks. It did not create
a Rust workspace, install an inference engine, download a model, or run a
benchmark.

The repository started from a single `Initial commit` containing only
`LICENSE` (Apache-2.0) and a one-line `README.md`. Both `main` and `dev` pointed
to that commit. No user changes were present.

## Completed

- Repository and environment reconnaissance recorded in the Stage 00 report.
- `main`/`dev` policy, Conventional Commits rules, commit template, local hook,
  commit-range checker, and CI workflow.
- Documentation checks for required files, relative links, ADR structure, and
  stage-report sections.
- Project charter with MVP, non-goals, success criteria, risks, and governance.
- Architecture proposal with domain boundaries and a delayed crate-split plan.
- Engine research with source URLs, access dates, and confidence levels.
- Compatibility and investigation matrix for official llama.cpp, CUDA 12.8,
  CUDA 13.x, Vulkan, CPU, and PrismML's fork.
- Benchmark methodology and report layout.
- Versioning, release-candidate, and promotion checklists.
- Installation, quick-start, troubleshooting, and uninstall documentation
  contracts that explicitly state that no release exists yet.
- Stage report template and Stage 00 report.

## Not completed / deferred

| Item | Reason | Target |
| --- | --- | --- |
| Rust workspace and `ghostram` binary | Stage 00 is documentation-only; crate split must follow ADR review | Stage 01 |
| Hardware probe implementation | Requires workspace and tests | Stage 01 |
| Engine installation/verification | Requires owner authorization and a pinned build | Stage 01/04 |
| GGUF metadata parser | Requires dependency evaluation and parser tests | Stage 02 |
| Memory estimator | Needs metadata and calibration data | Stage 03 |
| Engine process supervisor | Requires adapter contract and fake-engine tests | Stage 04 |
| Benchmark execution | No engine/model installed; no hardware tests authorized | Stage 05 |
| Tuner and history store | Depends on measurement pipeline | Stage 06 |
| Iced UI | Depends on stable core services | Stage 07 |
| Windows support | Linux-first scope | Later |
| Secondary GTX 1070 validation | Machine not accessed | Later, owner scheduled |

## Environment snapshot

These are observations, not guarantees that the values remain unchanged.

| Item | Observed value |
| --- | --- |
| OS | CachyOS Linux (rolling), kernel 7.2.11-1-cachyos, glibc 2.44 |
| CPU | Intel Core i7-8750H, 6C/12T, AVX2/FMA/F16C, governor `powersave` |
| RAM | 7.4 GiB total; ~3.2-3.5 GiB `MemAvailable` at idle inspection |
| Swap | 7.4 GiB zram (`zstd`), ~1.3 MiB used; no disk swap |
| GPU | NVIDIA GTX 1050 Ti Mobile, 4096 MiB, compute capability 6.1 |
| GPU driver | 580.178.04; `nvidia-smi` reports maximum CUDA runtime 13.0 |
| Display | Intel UHD 630, KDE Wayland; NVIDIA GPU idle during inspection |
| Vulkan | Loader 1.4.363; NVIDIA device Vulkan 1.4.312, Intel Mesa 26.2.4 |
| Storage | 41 GiB free on `/`; 773 GiB free on `/myfiles` |
| Rust | rustc/cargo 1.94.1, stable x86_64 |
| Missing tools | `cmake`, `ninja`, `nvcc`, CUDA toolkit, llama.cpp, Ollama, `gh` |
| Models | None found in the inspected common directories |

## Validation summary

| Check | Result |
| --- | --- |
| `python3 scripts/check_docs.py` | See Stage 00 report for the observed result |
| `bash scripts/check-commit-range.sh <base> HEAD` | See Stage 00 report |
| Commit-message pass/fail fixtures | See Stage 00 report |
| YAML parse of `.github/workflows/quality.yml` | See Stage 00 report |
| `git diff --check` | See Stage 00 report |
| Rust format/lint/test | Not applicable: no Cargo workspace |
| Hardware inference/benchmark | Not performed: no engine/model, Stage 00 forbids expensive tests |

## Known limitations and blockers

- No runnable GhostRAM artifact exists.
- No engine build has been verified on this laptop by the project.
- CUDA 13.x is unsuitable for Pascal; a CUDA 12.8 build or Vulkan must be used.
- The CUDA runtime libraries (`libcudart`, cuBLAS) are not installed locally;
  official CUDA binaries may require a separate runtime asset or a toolkit.
- The 4 GiB VRAM and 7.4 GiB RAM budget is tight; usable model sizes are unknown
  until measured.
- The secondary machine is not available.
- The GitHub repository is public, but no release, tag, or push was performed in
  Stage 00.

## Next stage

Recommended: **Stage 01 — Minimal Rust workspace and diagnostic spine.**

It should create only the crates justified by ADR-0002, implement a read-only
`ghostram doctor` report (OS, CPU, RAM, swap, GPU, driver, Vulkan devices, engine
discovery, model directories), add tests, and run on the primary laptop. It must
not install engines, download models, or implement the optimizer.

## Update rules

Update this file at the end of every stage. Record the actual branch, commit,
validation results, blockers, and next stage. Do not mark a stage accepted before
the human owner has reviewed it. Do not use this file as a substitute for the
detailed stage report.
