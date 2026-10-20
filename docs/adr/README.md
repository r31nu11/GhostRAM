# Architecture decision records

This directory records decisions that affect the project's architecture,
workflow, and evidence rules. An ADR is a durable explanation, not a substitute
for a stage report.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-branching-and-commit-policy.md) | Branching, commits, and stage workflow | Accepted |
| [0002](0002-modular-monolith-and-crate-split.md) | Modular monolith and delayed crate split | Accepted |
| [0003](0003-engine-integration-strategy.md) | Process-based engine integration, official llama.cpp first | Accepted |
| [0004](0004-measurement-provenance.md) | Separate estimates from measurements and preserve provenance | Accepted |
| [0005](0005-pinned-engine-builds.md) | Separate engine build identity from installation and capability observations | Accepted |

"Accepted" means the maintainer adopted the decision on 2026-10-20 as part of
Stage 00 acceptance. Accepted ADRs guide work until they are superseded or
deprecated. Decisions must be revisited when measurements show that an assumption
is wrong; a changed decision requires a superseding ADR, not a silent rewrite.

Do not record approval-history metadata in an ADR. Use the status field to show
whether a decision is proposed, adopted, rejected, superseded, or deprecated.

## Required structure

Every ADR contains:

- `Status:` — Proposed (pending adoption), Accepted, Rejected, Superseded, or
  Deprecated;
- `Date:` — the decision date in ISO 8601;
- `## Context` — the forces and evidence;
- `## Decision` — the choice;
- `## Consequences` — positive, negative, and follow-up effects.

Additional sections such as `## Alternatives considered`, `## References`, or
`## Migration` are encouraged when useful.

## Rules

1. Do not rewrite an accepted decision silently. Add a new ADR that supersedes it
   and update the old ADR's status with a link to the replacement.
2. Keep decisions specific enough to test. "Use good architecture" is not a
   decision.
3. Cite external evidence with an access date when the decision depends on
   upstream behavior.
4. Distinguish a hypothesis from a validated result. An ADR may choose an
   experiment; it may not declare the experiment successful.
5. Number ADRs sequentially. Do not reuse a number.
6. Update the index when an ADR is added or its status changes.
7. Keep decisions revisable. A measurement or integration result may justify a
   superseding ADR; do not silently rewrite an accepted decision.
8. The project maintainer is `r31nu11`. Commit identity and release approval
   follow [ADR-0001](0001-branching-and-commit-policy.md).

## Template

```markdown
# ADR-NNNN: Title

**Status:** Proposed (pending adoption)
**Date:** YYYY-MM-DD
**Deciders:** `r31nu11` (project maintainer)
**Related:** links to ADRs, research, or stage reports

## Context

What problem is being decided, what evidence exists, and what constraints apply?

## Decision

What is chosen? Be concrete about boundaries and non-goals.

## Consequences

What becomes easier, what becomes harder, and what must be revisited?

## Alternatives considered

What was rejected and why?

## References

Sources, experiments, or related documents.
```
