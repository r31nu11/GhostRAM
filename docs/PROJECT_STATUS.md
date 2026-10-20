# Project status

**Snapshot date:** 2026-10-20.
**Current stage:** Stage 00 — Technical Reconnaissance and Project Charter.
**Stage state:** Documentation and repository foundation complete; **awaiting
human review and acceptance**.
**Current branch:** `dev`.
**Stage 00 content commit (identity-corrected):**
`f0f9a8135715037b4dc81d4df0eb986ce90b6d69`.
**Stage 00 status/report commit (identity-corrected):**
`b964fad3ed8810833a8a415f06b21d0748fd2686`.
**Repository review correction commit:** the newest commit on `dev`; see
`git log --oneline --decorate -5`.
**Working tree:** clean after the review correction commit.
**Upstream:** not pushed; `dev` is ahead of `origin/dev`.

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
- Repository-maintenance pass: corrected the two unpublished Stage 00 commits to
  the owner's identity (`r31nu11`), preserved their trees/messages/order, and
  clarified branch, push, tag, and release authority.

## Owner decisions (2026-10-20)

- ADR-0001 through ADR-0005 are accepted in principle; line-by-line review and
  formal stage acceptance remain pending.
- No engine version/binary is pinned yet. Select the first baseline during Stage 04
  integration after build, checksum, dependency, device, and launch checks.
- Do not install an engine or model in Stage 01. The first installation and smoke
  test belong to Stage 04 or later with separate owner permission.
- The MVP detects externally installed engines; do not bundle engine binaries. A
  later voluntary download of a verified build into a managed directory may be
  considered.
- No universal speed threshold. Define usability criteria by hardware and task
  profile after real measurements.
- Test the GTX 1070 after the laptop diagnostic/execution chain works, before
  serious performance claims and the first stable release.
- `dev` is development/integration; `main` holds owner-accepted working/stable
  checkpoints. Push, tag, and GitHub Release remain separate owner decisions.

## Not completed / deferred

| Item | Reason | Target |
| --- | --- | --- |
| Rust workspace and `ghostram` binary | Stage 00 is documentation-only; crate split must follow ADR review | Stage 01 |
| Hardware probe implementation | Requires workspace and tests | Stage 01 |
| Engine baseline selection and pinning | Must follow Stage 04 build/checksum/dependency/device/launch checks | Stage 04 |
| Engine installation/verification | Requires separate owner authorization; not part of Stage 01 | Stage 04+ |
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
| `python3 scripts/check_docs.py` | Passed: 27 Markdown files, 104 relative links, 20 required files |
| `bash scripts/check-commit-range.sh bdfced9 HEAD` | Passed: all three commit messages in the review range accepted |
| Commit-message pass/fail fixtures | Valid accepted; invalid header rejected with exit 1 |
| YAML parse of `.github/workflows/quality.yml` | Passed: jobs `documentation` and `commit-messages` |
| `git diff --check` / cached check | Clean |
| Shell syntax (`bash -n`) | Passed for both scripts and the hook |
| Local Git identity | `r31nu11` / `r31nu11@mail.ru`; global config unchanged |
| Identity-corrected commits | Author and committer are `r31nu11 <r31nu11@mail.ru>` |
| Tree preservation | Old/new commit trees identical; messages, order, and author dates preserved |
| Remote state | `origin/dev` and `origin/main` still at `bdfced9`; no push |
| Rust format/lint/test | Not applicable: no Cargo workspace |
| Hardware inference/benchmark | Not performed: no engine/model; expensive tests not authorized |

## Known limitations and blockers

- No runnable GhostRAM artifact exists.
- No engine build has been verified on this laptop by the project.
- CUDA 13.x cannot build new Pascal device code; a CUDA 12.x toolchain is required
  for a Pascal-targeted CUDA build, with Vulkan as fallback. A CUDA 13.x-labeled
  prebuilt remains artifact-specific and unverified.
- The CUDA runtime libraries (`libcudart`, cuBLAS) are not installed locally;
  official CUDA binaries may require a separate runtime asset or a toolkit.
- The 4 GiB VRAM and 7.4 GiB RAM budget is tight; usable model sizes are unknown
  until measured.
- The secondary machine is not available.
- The GitHub repository is public, but no release, tag, or push was performed in
  Stage 00.

## Next stage

Recommended: **Stage 01 — Minimal Rust workspace and diagnostic spine.** Not
started and not yet authorized.

It should create only the crates justified by ADR-0002, implement a read-only
`ghostram doctor` report (OS, CPU, RAM, swap, GPU, driver, Vulkan devices, engine
discovery, model directories), add tests, and run on the primary laptop. It must
not install engines or models, download anything, launch inference, or implement
the optimizer. Engine installation and the first smoke test are deferred to Stage
04 or later with separate owner permission.

## Update rules

Update this file at the end of every stage. Record the actual branch, commit,
validation results, blockers, and next stage. Do not mark a stage accepted before
the human owner has reviewed it. Do not use this file as a substitute for the
detailed stage report.
