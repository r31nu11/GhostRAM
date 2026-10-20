# Architecture decision records

This directory records decisions that affect the project's architecture,
workflow, and evidence rules. An ADR is a durable explanation, not a substitute
for a stage report.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-branching-and-commit-policy.md) | Branching, commits, and stage workflow | Proposed |
| [0002](0002-modular-monolith-and-crate-split.md) | Modular monolith and delayed crate split | Proposed |
| [0003](0003-engine-integration-strategy.md) | Process-based engine integration, official llama.cpp first | Proposed |
| [0004](0004-measurement-provenance.md) | Separate estimates from measurements and preserve provenance | Proposed |
| [0005](0005-pinned-engine-builds.md) | Pin and fingerprint engine builds; probe capabilities per build | Proposed |

"Proposed" means the decision is part of the Stage 00 review package and has not
been accepted by the human owner. A proposed ADR is still binding for work done
under it until the owner changes it.

## Required structure

Every ADR contains:

- `Status:` — Proposed, Accepted, Rejected, Superseded, or Deprecated;
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

## Template

```markdown
# ADR-NNNN: Title

**Status:** Proposed
**Date:** YYYY-MM-DD
**Deciders:** human owner, project engineers
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
