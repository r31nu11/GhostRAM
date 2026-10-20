# GhostRAM repository policy

GhostRAM is maintained by `r31nu11` (GitHub account `r31nu11`). This file
defines the operating rules for contributors and automated coding tools. The
repository is the authoritative source of project state; external summaries are
not authoritative.

The project maintainer owns the repository-local Git identity. All commits must
use the maintainer's confirmed name and email. Do not commit as `GhostRAM Agent`,
`agent@localhost`, or another invented identity. If the local identity is
missing, placeholder-like, or unconfirmed, stop before committing and request
maintainer configuration.

## Mandatory start-of-work checklist

Before changing anything:

1. Inspect the working directory, current branch, `git status`, recent commits,
   remotes, and relevant diffs.
2. Read this file, `docs/PROJECT_STATUS.md`, `docs/ROADMAP.md`, and the
   specifications relevant to the current stage.
3. Read the preceding stage report under `reports/stages/`.
4. Verify the actual repository state instead of relying on an earlier summary.
5. Identify the exact scope and acceptance criteria for the current stage.
6. Confirm that unrelated work is preserved.
7. Report uncertainties and blockers instead of inventing missing context.

If stage instructions and the repository disagree, stop and request maintainer
direction. Do not silently rewrite an established decision. Review the relevant
ADR and propose a superseding ADR with reasons.

## Current state

- Stage 00 (technical reconnaissance and project charter) is complete pending
  formal acceptance. The technical direction of the ADRs has been reviewed in
  principle; all ADRs remain Proposed and adoption is pending.
- There is no Rust workspace, no GhostRAM binary, no installed inference engine,
  and no model catalog in this repository. Stage 01 has not started.
- No benchmark results exist. Never imply that a planned benchmark was executed.
- The primary laptop has a 4 GiB Pascal GPU and 7.4 GiB RAM. The secondary
  machine has not been accessed.

Read [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for the volatile snapshot
and [reports/stages/stage-00-technical-reconnaissance.md](reports/stages/stage-00-technical-reconnaissance.md)
for the Stage 00 evidence.

## Hard rules

- Work on `dev` or a temporary branch based on `dev`. Do not commit ordinary
  development to `main`.
- `main` receives only maintainer-accepted, working/stable checkpoints. Promotion
  from `dev` to `main` requires maintainer acceptance and applicable validation.
- Do not push, create a GitHub repository, or change remotes. Pushes are never
  automatic.
- Tags and GitHub Releases require **separate explicit maintainer approval**.
  Stage acceptance or a merge to `main` does not authorize either. A
  documentation-only stage is not a stable application release.
- Do not force-push, rewrite published history, reset destructively, or clean
  untracked files.
- Do not delete or overwrite user files, models, benchmarks, or unrelated work.
- Do not install packages, modify drivers, change system-wide settings, or run
  expensive benchmarks without explicit approval.
- Do not download large models. Do not silently download engines or runtimes.
- Do not fabricate measurements, test results, hardware observations, or source
  claims. State "not measured" when it was not measured.
- Do not claim that an architecture is validated because it is documented.
- Do not implement future-stage functionality early unless the stage explicitly
  allows it.
- Keep changes consistent with the existing style and documentation contracts.

## Repository map

| Path | Purpose |
| --- | --- |
| `README.md` | Public project overview and current status. |
| `AGENTS.md` | Repository policy and automation rules. |
| `docs/PROJECT_CHARTER.md` | Stable mission, MVP, non-goals, success criteria. |
| `docs/PROJECT_STATUS.md` | Volatile current state; update every stage. |
| `docs/ROADMAP.md` | Dependency-ordered stages; update every stage. |
| `docs/ARCHITECTURE.md` | Components, boundaries, and data flow. |
| `docs/ENGINE_SUPPORT.md` | Engine/backend compatibility matrix and confidence. |
| `docs/BENCHMARKING.md` | Measurement and reporting methodology. |
| `docs/TESTING.md` | Current checks and planned test strategy. |
| `docs/CONTRIBUTING.md` | Branch, commit, review, and documentation workflow. |
| `docs/RELEASING.md` | SemVer policy and release checklist. |
| `docs/INSTALL.md`, `docs/QUICK_START.md`, `docs/TROUBLESHOOTING.md`, `docs/UNINSTALL.md` | User-facing contracts; not yet executable instructions. |
| `docs/adr/` | Architecture decision records. |
| `docs/research/` | Dated research evidence with sources and confidence. |
| `reports/stages/` | Mandatory per-stage reports and template. |
| `reports/benchmarks/` | Curated benchmark reports; no fabricated data. |
| `scripts/` | Repository checks; never a substitute for real tests. |

## Workflow

1. Create a task branch from `dev` using a descriptive name such as
   `stage-01-workspace` or `feat/probe-cpu`.
2. Make coherent changes. Do not create artificial commits to reach a count.
3. Use Conventional Commits with the body sections described in
   [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).
4. Run the applicable checks and record exact commands and outcomes.
5. Update `docs/PROJECT_STATUS.md`, `docs/ROADMAP.md`, and the stage report.
6. Commit on the task branch using the maintainer's local identity, then merge or
   request maintainer merge into `dev`. Promotion of `dev` to `main` requires
   maintainer acceptance and applicable validation.
7. Report blockers, incomplete criteria, and decisions that require maintainer
   direction.
8. Do not tag or create a GitHub Release unless the maintainer has approved it
   separately from stage acceptance and promotion.

## Validation

Current Stage 00 checks:

```sh
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
git config --local --get user.name
git config --local --get user.email
```

The two `git config` commands must report the maintainer's confirmed identity,
never an automated or invented identity.

Future stages must add, at minimum:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run commands from the repository root. Never claim a command passed unless its
output was observed. If a command cannot run, document why.

## Documentation rules

- `README.md`: user-facing overview; update when capabilities or status change.
- `AGENTS.md`: concise repository and automation policy; update when the workflow
  changes.
- `PROJECT_CHARTER.md`: stable; change only with an explicit product decision.
- `PROJECT_STATUS.md`: update at the end of every stage.
- `ROADMAP.md`: update stage statuses and dependencies every stage.
- `ARCHITECTURE.md`: update when components, boundaries, or data flow change.
- `ENGINE_SUPPORT.md`: update with evidence, not assumptions; keep confidence
  labels.
- `BENCHMARKING.md`, `TESTING.md`: update when methods or commands change.
- `INSTALL.md` and user docs: do not invent functionality; label future behavior.
- ADRs: never rewrite an adopted decision silently; add a superseding ADR.
- Stage reports: mandatory, complete all 14 sections, and state what was not done.

## Stage completion

A stage is complete only when its acceptance criteria are explicitly checked, the
validation commands and observed results are recorded, the stage report is
written, `PROJECT_STATUS.md` and `ROADMAP.md` reflect reality, and the maintainer
has accepted the result. Code volume is not an acceptance criterion.

Stage acceptance does not promote `dev` to `main`, create a tag, publish a
release, or authorize a push. Those actions remain separate maintainer decisions.

When in doubt, preserve existing work, avoid destructive actions, and request
maintainer direction.
