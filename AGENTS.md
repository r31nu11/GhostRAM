# Instructions for AI agents

This file is the entry point for every AI agent working on GhostRAM. The
repository is the authoritative source of project state. Chat history and model
memory are not authoritative.

## Mandatory start-of-stage checklist

Before changing anything:

1. Inspect the working directory, current branch, `git status`, recent commits,
   remotes, and relevant diffs.
2. Read `AGENTS.md`, `docs/PROJECT_STATUS.md`, `docs/ROADMAP.md`, and the
   specifications relevant to the assigned stage.
3. Read the preceding stage report under `reports/stages/`.
4. Verify the actual repository state instead of trusting a previous chat summary.
5. Identify the exact scope and acceptance criteria for the assigned stage.
6. Confirm that unrelated user changes are preserved.
7. Report uncertainties and blockers instead of inventing missing context.

If the stage instructions and the repository disagree, stop and ask the human
owner. Do not silently rewrite an established decision. Review the relevant ADR
and propose a superseding ADR with reasons.

## Current state

- Stage 00 (technical reconnaissance and project charter) is complete pending
  human review.
- There is no Rust workspace, no GhostRAM binary, no installed inference engine,
  and no model catalog in this repository.
- No benchmark results exist. Never imply that a planned benchmark was executed.
- The primary laptop has a 4 GiB Pascal GPU and 7.4 GiB RAM. The secondary
  machine has not been accessed.

Read [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for the volatile snapshot
and [reports/stages/stage-00-technical-reconnaissance.md](reports/stages/stage-00-technical-reconnaissance.md)
for the Stage 00 evidence.

## Hard rules

- Work on `dev` or a temporary branch based on `dev`. Do not commit ordinary
  development to `main`.
- Do not push, create a GitHub repository, change remotes, tag, or publish a
  release unless the human owner explicitly authorizes it.
- Do not force-push, rewrite published history, reset destructively, or clean
  untracked files.
- Do not delete or overwrite user files, models, benchmarks, or unrelated work.
- Do not install packages, modify drivers, change system-wide settings, or run
  expensive benchmarks without explicit permission.
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
| `AGENTS.md` | Agent continuity rules and hard constraints. |
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
6. Commit on the task branch, then merge or ask the human owner to merge into
   `dev`. Promotion of `dev` to `main` is a human decision.
7. Report blockers, incomplete criteria, and decisions needed from the owner.

## Validation

Current Stage 00 checks:

```sh
python3 scripts/check_docs.py
bash scripts/check-commit-range.sh <base-sha> HEAD
```

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
- `AGENTS.md`: concise agent instructions; update when the workflow changes.
- `PROJECT_CHARTER.md`: stable; change only with an explicit product decision.
- `PROJECT_STATUS.md`: update at the end of every stage.
- `ROADMAP.md`: update stage statuses and dependencies every stage.
- `ARCHITECTURE.md`: update when components, boundaries, or data flow change.
- `ENGINE_SUPPORT.md`: update with evidence, not assumptions; keep confidence labels.
- `BENCHMARKING.md`, `TESTING.md`: update when methods or commands change.
- `INSTALL.md` and user docs: do not invent functionality; label future behavior.
- ADRs: never rewrite an accepted decision silently; add a superseding ADR.
- Stage reports: mandatory, complete all 14 sections, and state what was not done.

## Stage completion

A stage is complete only when its acceptance criteria are explicitly checked,
the validation commands and observed results are recorded, the stage report is
written, `PROJECT_STATUS.md` and `ROADMAP.md` reflect reality, and the human
owner has reviewed the result. Code volume is not an acceptance criterion.

When in doubt, preserve user work, avoid destructive actions, and ask the human
owner.
