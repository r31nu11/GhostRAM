# ADR-0001: Branching, commits, and stage workflow

**Status:** Proposed (pending adoption)
**Date:** 2026-10-20
**Deciders:** `r31nu11` (project maintainer)
**Related:** [CONTRIBUTING.md](../CONTRIBUTING.md), [ROADMAP.md](../ROADMAP.md), [Stage 00 report](../../reports/stages/stage-00-technical-reconnaissance.md)

## Context

The repository started from a single commit containing only `LICENSE` and a
one-line `README.md`. Stage 00 introduces a large body of documentation that must
remain reviewable by the maintainer and usable by later contributors without
relying on undocumented context.

The project needs:

- a stable branch for accepted checkpoints;
- an integration branch where stage work can be reviewed before promotion;
- readable history that states what changed, why, and what was actually checked;
- protection against accidental pushes, tags, releases, and destructive history
  edits;
- a workflow that works for a small project without adding a heavy process.

## Decision

1. `dev` is the development and integration branch for ordinary and intermediate
   commits.
2. `main` receives only maintainer-accepted, working/stable checkpoints. It
   changes only by maintainer-approved promotion from `dev`.
3. Promotion from `dev` to `main` requires maintainer acceptance and the
   applicable validation. It is not automatic.
4. Stage work may use short-lived task branches based on `dev`, named for the work
   (for example `stage-01-workspace` or `feat/probe-cpu`).
5. Commits use Conventional Commits with the body sections `Changes:`, `Reason:`,
   and `Validation:`. A breaking change requires `!` and a
   `BREAKING CHANGE:` footer.
6. Commit validation is available through an opt-in local `commit-msg` hook and a
   commit-range script used by CI.
7. Documentation checks and commit-range checks run in CI. Rust checks are added
   when a workspace exists.
8. Pushes are never automatic. Pushing, changing remotes, force-pushing, and
   rewriting published history require explicit maintainer authorization.
9. Tags and GitHub Releases require separate explicit maintainer approval. Stage
   acceptance, a merge to `main`, or a release candidate does not authorize them.
10. A documentation-only stage is not a stable application release.
11. Every stage ends with a report, updated status/roadmap, recorded validation,
    and maintainer review. Code volume is not an acceptance criterion.
12. All commits use the maintainer's repository-local Git identity `r31nu11` with
    the maintainer's confirmed email. Automated or invented identities such as
    `GhostRAM Agent` are not permitted.

## Consequences

Positive:

- History explains intent and evidence, not only file changes.
- The maintainer can review `dev` before promoting to `main`.
- Contributors and automated tools have a repository-based source of truth.
- Destructive or externally visible actions are gated.
- Authorship and release approval are unambiguous.

Negative:

- Every commit needs a small amount of structure and honest validation text.
- The workflow adds branch overhead for small documentation fixes.
- A local hook is opt-in; CI remains the message-format backstop, not an
  authorship or branch-permission enforcement mechanism.

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
