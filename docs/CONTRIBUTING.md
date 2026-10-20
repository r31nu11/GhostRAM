# Contributing

**Status:** Stage 00 workflow contract. The repository has no runnable
application code yet; the workflow below applies to documentation, research, and
future implementation stages.

## 1. Before you start

1. Read [AGENTS.md](../AGENTS.md), [PROJECT_STATUS.md](PROJECT_STATUS.md), and the
   relevant stage in [ROADMAP.md](ROADMAP.md).
2. Check `git status`, the current branch, and recent commits. Preserve unrelated
   work.
3. If a stage instruction conflicts with the repository, stop and ask the human
   owner. Do not silently rewrite an established decision.
4. Do not install packages, download models or engines, change drivers, or run
   expensive benchmarks without explicit permission.

## 2. Branch model

```text
main        accepted, reviewed checkpoints only
  │
  └── dev   integration branch for the current stage
        │
        └── task branch   short-lived work, merged back into dev
```

- `main` is promoted from `dev` only by the human owner.
- Ordinary development happens on `dev` or a task branch based on `dev`.
- Task branch names should describe the work, for example
  `stage-01-workspace`, `feat/probe-cpu`, or `fix/gguf-bounds`.
- Do not push, force-push, rewrite published history, tag, or publish a release
  without explicit owner authorization.
- Do not delete or clean untracked files that you did not create.

Keeping changes on a task branch is recommended when the work is large or
experimental. Small documentation corrections may be committed directly to
`dev`, provided the stage report and status are updated in the same change.

## 3. Commit convention

GhostRAM uses Conventional Commits:

```text
<type>(<scope>): <short imperative description>

Changes:
- <what changed>

Reason:
- <why the change is needed>

Validation:
- <exact command and observed result, or "not run: reason">
```

### Types

| Type | Use for |
| --- | --- |
| `feat` | New user-visible capability. |
| `fix` | Correction of a defect. |
| `perf` | Performance improvement with evidence. |
| `refactor` | Internal restructuring without behavior change. |
| `test` | Tests or fixtures. |
| `docs` | Documentation, research, or comments only. |
| `build` | Build system or dependency changes. |
| `ci` | Continuous integration changes. |
| `chore` | Repository maintenance that fits no other type. |
| `revert` | Reverting a previous commit. |

### Rules

- The header is at most 100 characters and does not end with a period.
- Use a lowercase, imperative subject after the type and optional scope.
- Leave a blank line after the header.
- Use `- ` bullets under each required section.
- `Validation:` must state what was actually run and observed. If nothing could
  be run, write `- Not run: <specific reason>`.
- A breaking change uses `!` after the type/scope and a
  `BREAKING CHANGE: <description>` footer.
- A revert commit may be header-only. If it has a body, it still uses the three
  sections.
- Do not add unrelated changes to a commit merely to make it larger.
- Do not add AI attribution unless the human owner asks for it; the commit must
  not claim a test or review that did not happen.

### Example

```text
docs(stage-00): establish engine research and project workflow

Changes:
- Add the engine support matrix and dated research notes.
- Document the branch, commit, testing, and release contracts.

Reason:
- Stage 00 needs a reviewable technical baseline before implementation.

Validation:
- python3 scripts/check_docs.py -> ok with the observed file/link counts.
- bash scripts/check-commit-range.sh <base> HEAD -> all commits accepted.
```

The example is illustrative. Do not copy its validation results into a real
commit without running the commands.

### Local hook

The repository ships an opt-in `commit-msg` hook:

```sh
git config core.hooksPath .githooks
```

This changes only the local repository configuration. Do not set it globally in
someone else's environment. The same check runs in CI through
`scripts/check-commit-range.sh`.

The helper can be tested directly:

```sh
bash scripts/check-commit-message.sh path/to/message-file
```

## 4. Review checklist

Before requesting review, verify:

- [ ] The change matches the current stage's scope and acceptance criteria.
- [ ] Unrelated user work is untouched.
- [ ] New behavior has tests, or the report states why it cannot be tested.
- [ ] `cargo fmt`, `cargo clippy`, and `cargo test` pass once a workspace exists.
- [ ] `python3 scripts/check_docs.py` passes.
- [ ] Commit messages in the range pass `scripts/check-commit-range.sh`.
- [ ] Documentation links resolve and claims are supported by evidence.
- [ ] No secrets, model weights, engine binaries, or personal data are committed.
- [ ] `PROJECT_STATUS.md` and `ROADMAP.md` reflect reality.
- [ ] The stage report is complete, including what was not done.
- [ ] No push, tag, release, or system change occurred without authorization.

The reviewer checks the evidence, not only the diff. A green check does not
compensate for a missing hardware test or an unsupported performance claim.

## 5. Documentation changes

- User-facing docs must label planned behavior as planned. Do not write commands
  for a binary that does not exist.
- Research notes carry an access date and distinguish observed, reported, and
  hypothetical claims. Use the confidence labels in
  [ENGINE_SUPPORT.md](ENGINE_SUPPORT.md).
- Architecture changes require an ADR. Do not rewrite an accepted ADR silently;
  add a superseding ADR with context, reasons, consequences, and migration.
- Stage reports use [the template](../reports/stages/TEMPLATE.md) and all 14
  sections.
- Do not fabricate measurements. Write "not measured" when nothing was measured.

## 6. Code style (future)

- Rust 2024 edition, stable toolchain unless an ADR pins otherwise.
- `cargo fmt` is authoritative for formatting.
- Clippy warnings are errors in CI (`-D warnings`).
- Public library APIs use typed errors and documented units.
- No `unsafe` without a documented safety argument and tests.
- No shell string construction for engine arguments; pass argument arrays.
- Keep domain logic out of CLI and UI adapters.

## 7. Merge and stage completion

1. Commit coherent changes on the task branch.
2. Run the applicable checks and record exact results.
3. Update the stage report, `PROJECT_STATUS.md`, and `ROADMAP.md`.
4. Merge into `dev`, or ask the owner to merge. Do not promote `dev` to `main`
   yourself.
5. A stage is complete only after the owner reviews the report and accepts the
   result.

## 8. Questions and blockers

Report uncertainties instead of inventing context. For a conflict between a
stage instruction and an accepted decision, link the ADR and propose a
superseding decision. For security issues, follow [SECURITY.md](../SECURITY.md)
instead of opening a public issue with exploit details.
