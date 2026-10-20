# ADR-0002: Modular monolith and delayed crate split

**Status:** Proposed
**Date:** 2026-10-20
**Deciders:** human repository owner, project engineers
**Related:** [ARCHITECTURE.md](../ARCHITECTURE.md), [ROADMAP.md](../ROADMAP.md)

## Context

The project brief maps a target architecture with components such as core,
hardware probe, model catalog, planner, tuner, engine adapter, runtime, store,
CLI, and UI. Creating one crate per box before any is implemented would produce
placeholder packages, circular dependency pressure, and tests that verify
nothing.

At the same time, a single crate with unrestricted modules would let hardware,
process, model, and UI concerns leak into each other. The most important
boundaries are already known:

- predictions must not be confused with measurements;
- engine-independent intent must not be replaced by engine flags;
- the control plane must not depend on engine internals;
- the planner should be deterministic and testable without a GPU;
- the CLI and UI are adapters, not domain owners.

## Decision

Build a **modular monolith** in one Cargo workspace. Create crates only when a
boundary has a real dependency, test, ownership, or build reason.

Stage 01 creates exactly:

1. `ghostram-core` — domain types, units, IDs, errors, contracts, versioned
   serialization envelopes.
2. `ghostram-probe` — read-only Linux hardware discovery.
3. `ghostram-cli` — command-line adapter.

Later crates:

- `ghostram-models` when the GGUF parser lands (Stage 02).
- `ghostram-engine` when there is a process to supervise (Stage 04).
- `ghostram-planner`, `ghostram-tuner`, `ghostram-store`, and `ghostram-ui` when
  their APIs and tests justify a crate.

Dependency rules:

- `core` has no OS-specific dependency and no engine dependency.
- `probe` may use OS APIs and `core`; it must not depend on the UI or engine.
- `models` may use a parser and `core`; it must not depend on `probe`.
- `engine` may use `core`, process/HTTP libraries, and filesystem paths; it must
  not depend on `models` or `planner` types beyond `core` contracts.
- `planner` is pure and takes metadata plus hardware snapshots.
- `cli` and `ui` contain no domain rules.
- No implicit global configuration.

The ten-crate target map is a design map, not a requirement to create ten
packages.

## Consequences

Positive:

- Less placeholder code and fewer artificial abstractions.
- Faster compilation and simpler dependency review early on.
- Boundaries are documented and enforced by module structure before crate
  boundaries are needed.
- Tests can target pure logic and fake engines without a GPU.

Negative:

- Some boundaries are conventions rather than compiler-enforced crate
  separation.
- A later split may require moving code and adjusting visibility.
- Contributors may be tempted to put everything in `core`; review must prevent
  that.

Follow-up:

- Reassess the split at Stages 02, 04, and 06.
- Record any new crate's responsibility and dependency direction before adding it.
- Do not create a crate solely to mirror a diagram.

## Alternatives considered

- **Create all ten crates immediately:** maximizes apparent structure but creates
  empty packages and premature APIs.
- **Single crate forever:** simplest build, but weakens boundaries and makes it
  hard to test or reuse the planner and probe independently.
- **Microservices or separate processes for each component:** unnecessary
  operational complexity for a local desktop application.

## References

- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [ROADMAP.md](../ROADMAP.md)
