# ADR-0001: Branching, commits, and stage workflow

**Status:** Proposed
**Date:** 2026-10-20
**Deciders:** human repository owner, project engineers
**Related:** [CONTRIBUTING.md](../CONTRIBUTING.md), [ROADMAP.md](../ROADMAP.md), [Stage 00 report](../../reports/stages/stage-00-technical-reconnaissance.md)

## Context

The repository started from a single commit containing only `LICENSE` and a
one-line `README.md`. Stage 00 introduces a large body of documentation that must
remain reviewable by a human owner and usable by later agents without relying on
chat history.

The project needs:

- a stable branch for accepted checkpoints;
- an integration branch where stage work can be reviewed before promotion;
- readable history that states what changed, why, and what was actually checked;
- protection against accidental pushes, tags, releases, and destructive history
  edits;
- a workflow that works for a small project without adding a heavy process.

## Decision

1. `main` is the accepted checkpoint branch. It changes only by human-approved
   promotion from `dev`.
2. `dev` is the integration branch for ordinary development.
3. Stage work may use short-lived task branches based on `dev`, named for the work
   (for example `stage-01-workspace` or `feat/probe-cpu`).
4. Commits use Conventional Commits with the body sections `Changes:`, `Reason:`,
   and `Validation:`. A breaking change requires `!` and a
   `BREAKING CHANGE:` footer.
5. Commit validation is available through an opt-in local `commit-msg` hook and a
   commit-range script used by CI.
6. Documentation checks and commit-range checks run in CI. Rust checks are added
   when a workspace exists.
7. Pushing, tagging, publishing, changing remotes, force-pushing, and rewriting
   published history require explicit human authorization.
8. Every stage ends with a report, updated status/roadmap, recorded validation, and
   human review. Code volume is not an acceptance criterion.

## Consequences

Positive:

- History explains intent and evidence, not only file changes.
- The owner can review `dev` before promoting to `main`.
- Agents have a repository-based source of truth.
- Destructive or externally visible actions are gated.

Negative:

- Every commit needs a small amount of structure and honest validation text.
- The workflow adds branch overhead for small documentation fixes.
- A local hook is opt-in; CI remains the backstop.

Follow-up:

- Add Rust format, lint, and test jobs to CI in Stage 01.
- Review whether task branches are worth the overhead once the workspace exists.
- Keep the commit convention documented in one place and link to it from stage
  reports.

## Alternatives considered

- **Single `main` branch:** simpler, but removes the review checkpoint and makes
  promotion policy meaningless.
- **Full GitFlow with release branches:** too heavy for a project with no releases
  and a very small team.
- **Free-form commit messages:** easier to write, but loses searchable intent and
  makes validation claims inconsistent.
- **No CI, local checks only:** faster, but easy to bypass and produces no
  independent record.

## References

- [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/)
- [AGENTS.md](../../AGENTS.md)
- `scripts/check-commit-message.sh`, `scripts/check-commit-range.sh`
- `.github/workflows/quality.yml`
