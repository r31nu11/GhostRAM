# Stage 00 — Technical reconnaissance and project charter

**Date:** 2026-10-20
**Owner:** human repository owner
**Author:** AI engineering agent under owner authority
**Branch:** `dev`
**Status:** Complete pending human review and acceptance

## 1. Stage identifier and title

Stage 00 — Technical reconnaissance and project charter.

This is the foundation stage: it establishes the project contract, records the
actual starting environment, researches engine/backend options, defines the
architecture and evidence rules, and creates the repository workflow. It does not
implement the product.

## 2. Objectives and acceptance criteria

Stage 00 was assigned as a reconnaissance and charter stage. The acceptance
criteria below are derived from the assignment, the project charter, and the
repository rules. They are assessed against the actual repository state and the
validation recorded in section 7.

| # | Criterion | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Inspect the real repository and environment instead of assuming a blank project. | Met | Starting commit/branch/remotes, file inventory, OS/CPU/RAM/GPU/Vulkan/storage/tool observations recorded in the research notes and this report. |
| 2 | Research at least one viable engine/backend path with sources and confidence labels. | Met | Official llama.cpp, CUDA 12.8/13.4, Vulkan, CPU, and PrismML fork research with upstream sources and confidence labels. |
| 3 | Define the product mission, MVP, non-goals, success criteria, risks, and governance. | Met | [PROJECT_CHARTER.md](../../docs/PROJECT_CHARTER.md). |
| 4 | Propose architecture boundaries that separate estimates, measurements, capabilities, and the engine process. | Met | [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) and ADR-0002 through ADR-0005. |
| 5 | Define a reproducible benchmark methodology without fabricating results. | Met | [BENCHMARKING.md](../../docs/BENCHMARKING.md); this report states no benchmark was run. |
| 6 | Define branch, commit, review, testing, and release policies. | Met | [CONTRIBUTING.md](../../docs/CONTRIBUTING.md), [TESTING.md](../../docs/TESTING.md), [RELEASING.md](../../docs/RELEASING.md), ADR-0001, commit scripts, CI workflow. |
| 7 | Provide user-facing installation/quick-start/troubleshooting/uninstall contracts that do not pretend the product exists. | Met | User docs explicitly label all behavior as planned and provide no false install commands. |
| 8 | Create ADRs for the consequential Stage 00 decisions. | Met | ADR-0001 through ADR-0005, all proposed for owner review. |
| 9 | Provide a mandatory 14-section stage report and update status/roadmap. | Met | This report; [PROJECT_STATUS.md](../../docs/PROJECT_STATUS.md); [ROADMAP.md](../../docs/ROADMAP.md). |
| 10 | Do not install an engine, download a model, run an expensive benchmark, change system settings, push, tag, or release. | Met | No engine/model exists; no installation or benchmark command was run; no push/tag/release was performed. |
| 11 | Validate the documentation and workflow checks and record exact results. | Met | Section 7 records the commands and observed outputs. |
| 12 | Human owner reviews and accepts the stage. | Pending | This report is submitted for review. The stage is not marked accepted. |

The stage is **not accepted** until the human owner completes the line-by-line
review of this report and the ADRs, records any further corrections, and accepts
the stage. Section 10 records the owner's decisions and the remaining questions.

## 3. Summary of completed and incomplete work

### Completed

- Repository reconnaissance: current branch, remotes, history, starting files,
  and absence of prior user changes.
- Environment reconnaissance: OS, kernel, CPU, RAM, swap/zram, GPU, VRAM, driver,
  Vulkan devices, storage, toolchain, and model/engine absence.
- Research on official llama.cpp, CUDA 12.8 vs 13.x on Pascal, Vulkan, CPU
  fallback, GGUF, and the PrismML fork, with dated sources and confidence labels.
- Product charter with mission, users, MVP, non-goals, success criteria, risks,
  and governance.
- Architecture proposal with a modular-monolith stance and an explicit initial
  crate set.
- Engine integration strategy: process-based, official llama.cpp first, CPU
  baseline, CUDA 12.x or Vulkan on Pascal, PrismML later and optional.
- Benchmark methodology with provenance, controls, statistics, and reporting
  rules. No benchmarks were run.
- Testing strategy with current checks, future test layers, and hardware-test
  limits.
- Branch/commit/review/release policy and Conventional Commits tooling.
- User-facing installation, quick-start, troubleshooting, and uninstall contracts
  that clearly state they are not executable yet.
- Five ADRs covering workflow, crate decomposition, engine integration, measurement
  provenance, and build pinning.
- Stage report template, this report, and a benchmark report contract.
- Documentation/link/ADR/report checks and a CI workflow for docs and commit
  messages.
- Repository-maintenance pass: corrected the two unpublished Stage 00 commits to
  the owner's identity, verified their trees were unchanged, and aligned branch,
  push, tag, and release policy with the owner's decisions.

### Incomplete or deferred

| Item | Reason | Target |
| --- | --- | --- |
| Rust workspace and CLI | Stage 00 is documentation-only; crate split needs ADR review | Stage 01 |
| Hardware probe implementation | Requires workspace and tests | Stage 01 |
| Engine baseline selection and pinning | Requires Stage 04 build/checksum/dependency/device/launch checks | Stage 04 |
| Engine installation and capability probe | Requires separate owner authorization; not part of Stage 01 | Stage 04+ |
| GGUF parser | Requires dependency audit and malformed-input tests | Stage 02 |
| Memory planner | Requires metadata and later calibration | Stage 03 |
| Engine supervisor | Requires adapter contract and fake-engine tests | Stage 04 |
| Benchmark execution | No engine/model installed; expensive tests not authorized | Stage 05 |
| Tuner/history | Depends on measurement pipeline | Stage 06 |
| Iced UI | Depends on stable core services | Stage 07 |
| Windows support | Linux-first scope | Later |
| GTX 1070 validation | Secondary machine not accessed | Later, owner scheduled |

## 4. Architectural decisions and rationale

All decisions remain **Proposed** for formal purposes. On 2026-10-20 the owner
accepted their direction in principle and requested that decisions remain
revisable when measurements arrive; line-by-line review and formal acceptance are
still pending. They are binding for current work but are not fully accepted
architecture.

| ADR | Decision | Rationale |
| --- | --- | --- |
| [0001](../../docs/adr/0001-branching-and-commit-policy.md) | `dev` development/integration; `main` owner-accepted stable checkpoints; task branches; Conventional Commits with `Changes`/`Reason`/`Validation`; owner identity; no automatic push; separate tag/release approval. | Reviewability, continuity across agents, clear authority, and a durable record of what was validated. |
| [0002](../../docs/adr/0002-modular-monolith-and-crate-split.md) | Start with `core`, `probe`, and `cli`; split other crates when a real boundary justifies it. | Avoids placeholder crates and premature APIs while keeping testable boundaries. |
| [0003](../../docs/adr/0003-engine-integration-strategy.md) | External child process + HTTP/CLI; official llama.cpp first; CPU baseline; CUDA 12.x or Vulkan on Pascal; PrismML later; no engine pinned or installed in Stage 01; detect external engines in the MVP. | Crash isolation, build variance, evidence-based scope control, and avoiding unverified binaries. |
| [0004](../../docs/adr/0004-measurement-provenance.md) | Separate `Estimate` and `Measurement`; run manifests; immutable records; labeled claims. | Prevents estimated or cherry-picked numbers from becoming product claims. |
| [0005](../../docs/adr/0005-pinned-engine-builds.md) | Pin/fingerprint builds and probe capabilities per build; no baseline version is pinned until Stage 04 integration checks. | "llama.cpp" is not one behavior; support must refer to an actual executable and device, verified before pinning. |

The architecture proposal also keeps the control plane and inference plane in
separate failure domains, and treats `--fit`-resolved values as part of the
experiment record rather than an invisible default.

## 5. Files added, changed, or removed

### Added

| Path | Purpose |
| --- | --- |
| `.editorconfig` | Consistent whitespace and line endings. |
| `.gitignore` | Excludes build output, local data, models, and secrets. |
| `.gitmessage` | Conventional Commit template. |
| `.githooks/commit-msg` | Opt-in local commit-message validation. |
| `.github/workflows/quality.yml` | Documentation and commit-range CI. |
| `AGENTS.md` | Agent continuity rules and hard constraints. |
| `CHANGELOG.md` | Keep a Changelog structure; no released version. |
| `SECURITY.md` | Security disclosure policy; no supported release yet. |
| `docs/PROJECT_CHARTER.md` | Mission, MVP, non-goals, risks, governance. |
| `docs/PROJECT_STATUS.md` | Volatile stage/branch/blocker snapshot. |
| `docs/ROADMAP.md` | Dependency-ordered stages 00–08. |
| `docs/ARCHITECTURE.md` | Components, boundaries, data flow, safety governor. |
| `docs/ENGINE_SUPPORT.md` | Engine/backend matrix with confidence labels. |
| `docs/BENCHMARKING.md` | Measurement and reporting methodology. |
| `docs/TESTING.md` | Current checks and planned test layers. |
| `docs/CONTRIBUTING.md` | Branch, commit, review, and documentation workflow. |
| `docs/RELEASING.md` | SemVer policy and release checklist. |
| `docs/INSTALL.md` | Installation contract; no executable instructions. |
| `docs/QUICK_START.md` | Planned first-run flow; not available. |
| `docs/TROUBLESHOOTING.md` | Environment constraints and safe diagnostics. |
| `docs/UNINSTALL.md` | Planned data locations and removal rules. |
| `docs/adr/README.md` | ADR index, structure, and rules. |
| `docs/adr/0001-branching-and-commit-policy.md` | Workflow decision. |
| `docs/adr/0002-modular-monolith-and-crate-split.md` | Crate decomposition decision. |
| `docs/adr/0003-engine-integration-strategy.md` | Engine boundary decision. |
| `docs/adr/0004-measurement-provenance.md` | Evidence model decision. |
| `docs/adr/0005-pinned-engine-builds.md` | Build pinning decision. |
| `docs/research/2026-10-20-engine-landscape.md` | Dated evidence and sources. |
| `reports/stages/TEMPLATE.md` | Mandatory 14-section stage report template. |
| `reports/stages/stage-00-technical-reconnaissance.md` | This report. |
| `reports/benchmarks/README.md` | Benchmark report contract; no results. |
| `scripts/check_docs.py` | Required files, links, ADR fields, report sections. |
| `scripts/check-commit-message.sh` | One-message convention check. |
| `scripts/check-commit-range.sh` | Range/branch policy check. |

### Changed

| Path | Change |
| --- | --- |
| `README.md` | Expanded from a one-line placeholder to the public project overview, current state, docs index, and safety statement. |

### Removed

None. No user files were deleted or overwritten.

## 6. Dependencies added or modified

- **No Cargo dependencies were added.** There is no `Cargo.toml` or Rust
  workspace.
- **No system packages were installed.** `cmake`, `ninja`, `nvcc`, the CUDA
  toolkit, llama.cpp, and models remain absent.
- **No Python packages were installed.** `scripts/check_docs.py` uses only the
  Python standard library.
- **No engine or model was downloaded.**
- The only external dependencies are documentation references (URLs) and the
  tools already present on the machine.
- Git hooks are opt-in and local; no global Git configuration was changed.

## 7. Validation commands and observed results

The final validation pass is recorded below. Commands were run from the repository
root. A result is not claimed unless it was observed.

### 7.1 Documentation integrity

```text
$ python3 scripts/check_docs.py
ok: 27 Markdown files, 104 relative links, 20 required files
```

### 7.2 Commit-message checker

Valid fixture and invalid fixtures were created under `/tmp/opencode`, outside the
repository.

```text
$ bash scripts/check-commit-message.sh /tmp/opencode/commit-valid.txt
ok: commit message accepted

$ bash scripts/check-commit-message.sh /tmp/opencode/commit-invalid-header.txt
error: invalid header: Docs: bad header format
expected: <type>(<scope>): <lowercase imperative description>
allowed types: feat fix perf refactor test docs build ci chore revert
exit=1

$ bash scripts/check-commit-message.sh /tmp/opencode/commit-missing-section.txt
error: missing required section: Validation:
exit=1

$ bash scripts/check-commit-message.sh /tmp/opencode/commit-breaking-no-footer.txt
error: breaking-change header requires a "BREAKING CHANGE: ..." footer
exit=1
```

### 7.3 Shell syntax

```text
$ bash -n scripts/check-commit-message.sh scripts/check-commit-range.sh .githooks/commit-msg
shell syntax: ok
```

### 7.4 Workflow YAML parse

```text
$ python3 -c 'import yaml; data=yaml.safe_load(open(".github/workflows/quality.yml", encoding="utf-8")); assert "jobs" in data and "documentation" in data["jobs"] and "commit-messages" in data["jobs"]; print("yaml parse: ok; jobs:", ", ".join(data["jobs"]))'
yaml parse: ok; jobs: documentation, commit-messages
```

### 7.5 Whitespace and status

After the identity-corrected status/report commit `b964fad`:

```text
$ git diff --check
git diff --check: clean

$ git status --short --branch
## dev...origin/dev [ahead 2]
```

### 7.6 Commit range

```text
$ bash scripts/check-commit-range.sh bdfced9d7db49a1e2bdad5dd1602822734da33b1 HEAD
check: f0f9a8135715 docs(stage-00): establish project foundation and engine research
ok: commit message accepted
check: b964fad3ed88 docs(stage-00): record validation results and content commit
ok: commit message accepted
ok: all checked commit messages passed
```

### 7.7 Rust checks

Not run. There is no `Cargo.toml` or Rust source in Stage 00:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

These commands are required from Stage 01 onward. Running them now would only
report that no manifest exists.

### 7.8 Commit identity correction

The two local Stage 00 commits were rewritten to the owner's configured identity
after confirming that `origin/dev` and `origin/main` still pointed at the starting
commit `bdfced9`. The starting commit was not altered.

```text
$ git log --format='%h %an <%ae> | %cn <%ce> | %s' bdfced9..HEAD
b964fad r31nu11 <r31nu11@mail.ru> | r31nu11 <r31nu11@mail.ru> | docs(stage-00): record validation results and content commit
f0f9a81 r31nu11 <r31nu11@mail.ru> | r31nu11 <r31nu11@mail.ru> | docs(stage-00): establish project foundation and engine research

$ git diff --stat df0ad8a f0f9a81
(no output; tree identical)

$ git diff --stat b75c3b7 b964fad
(no output; tree identical)
```

Identity rewriting changes commit hashes. The old hashes are recorded here and in
section 11 for traceability; they are no longer reachable from `dev` except
through the local reflog. The author dates and commit messages were preserved;
committer dates changed as expected. No commit was pushed.

## 8. Hardware tests and measurements

**None.** No engine was installed, no model was downloaded, and no inference or
benchmark was run.

The environment observations in the research notes are read-only inventory, not
performance measurements. There are no throughput, latency, memory-peak, or
quality numbers for GhostRAM.

The following remain unmeasured:

- whether the official CUDA 12.8 binary initializes on compute capability 6.1;
- whether it uses PTX JIT or native SASS;
- CUDA vs Vulkan performance on this GPU;
- which Vulkan device selector reliably picks the NVIDIA GPU;
- model fit and usable context under the RAM/VRAM budget;
- PrismML low-bit and speculative paths on Pascal;
- the secondary GTX 1070 machine's behavior.

## 9. Tests that could not be performed

| Test | Missing prerequisite | Risk |
| --- | --- | --- |
| Rust format/lint/unit tests | No workspace or source | Stage 01 must establish them; no application behavior is covered today. |
| Hardware probe tests | No probe implementation | Hardware parsing remains unvalidated. |
| Engine discovery/capability tests | No engine installed | No real build can confirm the adapter contract. |
| CUDA 12.8 execution | No toolkit/runtime or engine artifact; installation not authorized | The key Pascal compatibility hypothesis is unverified. |
| Vulkan execution | No engine build; no benchmark authorization | Device selection and performance are unverified. |
| CPU inference | No model or engine | Baseline throughput and memory are unverified. |
| GGUF parser tests | No parser selected | Model metadata handling remains unvalidated. |
| Process supervision tests | No implementation | Crash/timeout/orphan behavior is unverified. |
| Benchmark reproducibility | No harness, engine, or model | No measurement claim can be made. |
| Secondary GTX 1070 validation | Machine not accessed | Cross-machine transfer is unknown. |
| CI execution on GitHub | No push authorized | The workflow has been parsed locally, not executed by GitHub Actions. |

## 10. Known bugs, limitations, and unresolved questions

### Limitations

- There is no runnable GhostRAM artifact; the repository is documentation and
  repository tooling only.
- The ADRs are proposed and accepted in principle, not fully accepted; line-by-line
  review is pending.
- No engine build has been verified on this laptop by the project.
- No engine version is pinned; baseline selection is deferred to Stage 04.
- CUDA 13.x is unsuitable for Pascal; a CUDA 12.8 build or Vulkan is required.
- `libcudart` and cuBLAS are not installed locally; the official CUDA archive may
  require a separate runtime asset.
- The 4 GiB VRAM / 7.4 GiB RAM budget is tight; usable model sizes are unknown.
- The desktop and zram consume memory and complicate measurement.
- Two Vulkan devices are visible; selection must be explicit.
- The secondary GTX 1070 machine is not available.
- The GitHub repository is public, but nothing was pushed, tagged, or released in
  Stage 00. The identity correction did not push anything.
- The Stage 00 report's validation was performed on the local machine only; CI
  has not run.

### Owner decisions recorded 2026-10-20

| Question | Decision |
| --- | --- |
| ADR-0001 through ADR-0005 | Accept the directions in principle, fix remarks, and keep decisions revisable when measurements arrive. Full line-by-line review is pending. |
| First engine build | Pin none yet. Choose the version/binary in the integration stage after verifying build, checksum, dependencies, and launch. |
| Engine/model installation | Not in Stage 01. First installation and smoke test in Stage 04 or later, after the adapter exists, with separate owner permission. |
| Engine bundling | Do not bundle binaries in the MVP. Detect external installations first; later consider voluntary download of a verified build into a managed directory. |
| Usability threshold | No universal number without measurements. Collect laptop results first, then define criteria by hardware and task profile. |
| GTX 1070 testing | After the basic diagnostic/execution chain works on the laptop, before serious performance claims and the first stable release. |
| Linux packaging | Still open; decide during Stage 08. |

### Remaining open questions

1. Which exact engine build passes the Stage 04 integration checks and becomes the
   first pinned baseline?
2. What measured thresholds define a practical result for each hardware/task
   profile after Stage 05 data exists?
3. Which Linux packaging format and update channel should Stage 08 use?
4. When will the GTX 1070 machine be physically available for validation?

## 11. Current branch and commit hash

- **Branch:** `dev`.
- **Starting commit:** `bdfced9d7db49a1e2bdad5dd1602822734da33b1`
  (`Initial commit`).
- **Stage 00 content commit (identity-corrected):**
  `f0f9a8135715037b4dc81d4df0eb986ce90b6d69`
  (`docs(stage-00): establish project foundation and engine research`).
- **Stage 00 status/report commit (identity-corrected):**
  `b964fad3ed8810833a8a415f06b21d0748fd2686`
  (`docs(stage-00): record validation results and content commit`).
- **Repository-maintenance commit:** the follow-up commit containing this
  identity and branch-policy maintenance snapshot; see
  `git log --oneline --decorate -4`.
- **Push:** not performed. `origin/dev` and `origin/main` remained at
  `bdfced9d7db49a1e2bdad5dd1602822734da33b1` during the maintenance task.
- **Tag/release:** none.

The identity correction rewrote the two unpublished Stage 00 commits, so their
hashes changed from the original `df0ad8a`/`b75c3b7` pair to
`f0f9a81`/`b964fad`. The starting owner commit `bdfced9` was not altered. A commit
cannot contain its own hash; the final hand-off states the resulting HEAD.

## 12. Git working-tree status

At the start of Stage 00, the tree was clean on `dev` at `bdfced9` with no
untracked files.

After the identity-corrected status/report commit `b964fad`, the tree was clean
and `dev` was two commits ahead of `origin/dev`:

```text
## dev...origin/dev [ahead 2]
```

The repository-maintenance commit adds only the identity, branch-policy, owner
decision, and validation updates described in this report. The final hand-off
records the clean status after that commit. No unrelated user changes were
present. No files were removed, and nothing was pushed.

## 13. Documentation updated

Updated or created:

- `README.md`
- `AGENTS.md`
- `CHANGELOG.md`
- `SECURITY.md`
- `docs/PROJECT_CHARTER.md`
- `docs/PROJECT_STATUS.md`
- `docs/ROADMAP.md`
- `docs/ARCHITECTURE.md`
- `docs/ENGINE_SUPPORT.md`
- `docs/BENCHMARKING.md`
- `docs/TESTING.md`
- `docs/CONTRIBUTING.md`
- `docs/RELEASING.md`
- `docs/INSTALL.md`
- `docs/QUICK_START.md`
- `docs/TROUBLESHOOTING.md`
- `docs/UNINSTALL.md`
- `docs/adr/README.md` and ADR-0001 through ADR-0005
- `docs/research/2026-10-20-engine-landscape.md`
- `reports/stages/TEMPLATE.md`
- `reports/stages/stage-00-technical-reconnaissance.md`
- `reports/benchmarks/README.md`

The maintenance pass also updated `AGENTS.md`, `README.md`, `CHANGELOG.md`,
`docs/CONTRIBUTING.md`, `docs/RELEASING.md`, `docs/PROJECT_CHARTER.md`,
`docs/ENGINE_SUPPORT.md`, `docs/adr/README.md`, and ADR-0001 through ADR-0005 to
align ownership, branch, push, tag, and release policy and record the owner's
decisions. ADR-0002 and ADR-0004 received the owner-direction note; ADR-0001,
ADR-0003, and ADR-0005 also received policy or engine-baseline corrections.

`PROJECT_STATUS.md` and `ROADMAP.md` reflect Stage 00 as complete pending full
review, with Stage 01 as the next recommended but unauthorized stage. The
changelog records the documentation foundation and maintenance pass under
`Unreleased`, not a release.

## 14. Recommended next stage

**Stage 01 — Minimal Rust workspace and diagnostic spine.** Not started and not
authorized.

Recommended scope:

- create only `ghostram-core`, `ghostram-probe`, and `ghostram-cli`;
- implement a read-only `ghostram doctor` for OS/kernel, CPU, RAM, swap/zram, GPU,
  driver, Vulkan devices, storage, tool availability, and engine discovery;
- add typed units, errors, and a versioned JSON envelope;
- add parser fixtures and unit tests;
- run `cargo fmt`, `cargo clippy -D warnings`, and `cargo test`;
- add Rust jobs to CI;
- update the stage report, status, and roadmap.

Stage 01 must not install an engine or model, download anything, launch inference,
or implement the optimizer. Engine installation and the first real hardware smoke
test need separate owner authorization and belong in Stage 04 at the earliest.
The first baseline build is selected there after build, checksum, dependency,
device, and launch checks.

The owner must first complete the line-by-line ADR review, accept this report, and
authorize Stage 01. The owner decisions in section 10 remove the earlier
build/model approval ambiguity but do not start Stage 01.

## Hand-off checklist

- [x] Repository and environment inspected rather than assumed.
- [x] Engine/backend research recorded with sources and confidence.
- [x] Charter, architecture, methodology, and user-doc contracts written.
- [x] Workflow and repository checks created.
- [x] ADRs written and marked proposed.
- [x] No engine, model, package, driver, or system setting changed.
- [x] No push, tag, release, merge, branch creation, or branch deletion performed.
- [x] No benchmark result fabricated or implied.
- [x] Unpublished Stage 00 commits corrected to the owner's identity with trees,
  messages, and order preserved.
- [x] Branch, push, tag, and release policy clarified.
- [x] Owner decisions recorded.
- [x] Validation results filled in after the final command run.
- [ ] Human owner line-by-line review and Stage 00 acceptance.
- [ ] Separate authorization to start Stage 01.
