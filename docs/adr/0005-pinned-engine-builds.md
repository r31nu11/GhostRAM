# ADR-0005: Separate engine build identity from installation and capability observations

**Status:** Proposed (pending adoption)
**Date:** 2026-10-20
**Deciders:** `r31nu11` (project maintainer)
**Related:** [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md), [ARCHITECTURE.md](../ARCHITECTURE.md), [ADR-0003](0003-engine-integration-strategy.md)

## Context

"llama.cpp" is not one behavior. Its flags, backend support, model formats, and
performance change across commits, releases, and build configurations. On the
primary laptop, the difference is material:

- an official CUDA 12.8 build and a CUDA 13.4 build target different GPU
  architectures;
- a Vulkan build may see both Intel and NVIDIA devices;
- a CPU build has no CUDA runtime dependency but a very different performance
  profile;
- a fork can add model types or speculative-decoding paths that upstream does not
  have;
- a flag can appear in `--help` while the selected backend or device does not
  implement the corresponding path.

A test that says "works with latest llama.cpp" is not reproducible. A support
matrix that says "CUDA is supported" does not tell the user whether a particular
executable contains code for compute capability 6.1.

Three concepts need to remain distinct:

1. **Build identity** — for a single-executable artifact, the executable's
   SHA-256. Engine family, version, release tag, source commit, artifact name,
   target platform, build configuration, and dependency information are
   associated metadata, not part of the identifier.
2. **Installation record** — where a local copy was found, how it was discovered,
   and other installation-specific details.
3. **Capability observation** — a timestamped result produced by probing that
   build in a specific runtime context.

If a path or a device probe is folded into build identity, moving an executable
or probing the same executable on another device appears to create a different
build. That makes reproducibility and support claims depend on local filesystem
layout and on a single observation.

Provenance is often incomplete when a build is discovered externally. A release
tag, source commit, or build configuration may be unknown at first and confirmed
later. If those fields were inputs to the identifier, enriching or correcting
metadata would change the identity of an unchanged executable and split its
measurement history. The identifier therefore follows file content. Metadata may
be added or corrected and recorded as a provenance revision, not as a new build.

This ADR defines identity only for a single-executable artifact. If a future
supported engine artifact consists of multiple files, a deterministic
artifact-manifest fingerprint may be required. That schema is not defined here
and must not be inferred from this ADR.

## Decision

1. For a single-executable artifact, `EngineBuildId` is the executable's
   SHA-256. The SHA-256 is the canonical file identity. Do not include the
   executable path, provenance metadata, or runtime probe output in the
   identifier.
2. Record engine family, version, release tag, source commit, artifact name,
   target platform, build configuration, and dependency information as
   associated metadata. These fields describe and qualify the build but are not
   inputs to `EngineBuildId`. Discovering, adding, or correcting them must not
   silently change the identity of an unchanged executable; record the change as
   a provenance revision tied to the same build.
3. A build with unknown provenance keeps the same `EngineBuildId`. Unknown fields
   remain explicitly unknown until evidence supports a provenance revision. Do
   not invent metadata to complete a record.
4. If a future supported engine artifact consists of multiple files, introduce a
   deterministic artifact-manifest fingerprint as the identity for that artifact
   kind. This ADR does not define, invent, or implement that schema; a later
   decision is required. Until then, `EngineBuildId` applies only to
   single-executable artifacts.
5. Store an **installation record** separately. It records the executable path,
   discovery method (configured path, `PATH`, user-specified path, package, or
   other local source), installation details, and observation time. An
   installation record may change without changing build identity.
6. Store **capability observations** separately and timestamp them. Each
   observation is tied to an `EngineBuildId` and records the runtime context:
   backend, selected device, driver/runtime version, relevant dependency
   evidence, the probe command or endpoint, the result, and the supporting
   evidence.
7. Model each capability result as `supported`, `unsupported`, or `unknown`. A
   failed, missing, or inconclusive probe yields `unknown`, never `supported`.
   An observation from one backend, device, or driver does not automatically
   apply to another.
8. Record the selected backend and device explicitly, including the device
   selector used when multiple Vulkan adapters exist.
9. Keep CPU, CUDA 12.x, CUDA 13.x, and Vulkan as separate configurations. Do not
   merge their results into a single backend row.
10. Record whether a CUDA result used native SASS or PTX JIT when that can be
    determined. Do not assume the prebuilt architecture list from the engine
    family.
11. For externally discovered builds, record a release tag or source commit when
    available. Do not block discovery on unavailable provenance. Preserve the
    executable hash and known metadata, and label unknown provenance explicitly.
    Never invent a release, commit, version, or build configuration.
12. Do not auto-update an engine build during a benchmark or silently substitute
    a different build.
13. Treat a fork as a distinct build profile even when it reuses the same adapter
    code.
14. **Do not choose or pin a specific engine version in Stage 00 or Stage 01.**
    Select the first baseline during Stage 04 integration, after verifying build
    identity, checksum, runtime dependencies, device visibility, launch behavior,
    and the relevant capability observations. Until then, support claims remain
    unknown rather than tied to an assumed version.

## Consequences

Positive:

- Build identity remains stable when provenance is discovered, enriched, or
  corrected.
- Results can be reproduced and compared without depending on a local install
  path.
- Copying or relocating an executable does not fragment build history.
- A probe on one device does not silently become a claim about another device,
  backend, or driver.
- Build provenance, local installation details, and runtime evidence remain
  independently auditable.
- Multiple installations of the same build can coexist without corrupting
  measurement history.

Negative:

- The data model needs distinct build identity, associated metadata revisions,
  installation records, and capability observations, with explicit links between
  them.
- Externally discovered builds may have incomplete provenance; unknown fields
  must remain visible rather than inferred, and later corrections must preserve
  the existing build identity.
- A single-file SHA-256 does not identify a multi-file artifact; that case is
  deferred to a separate decision.
- Capability observations accumulate over time and need retention and selection
  rules.
- More metadata must be collected and validated.

Follow-up:

- Define how associated metadata and provenance revisions are recorded in Stage
  01 without installing an engine.
- Do not define a multi-file manifest schema until a supported artifact requires
  it; treat that as a separate decision.
- Capture probe fixtures from pinned official builds only when an engine is
  installed with separate maintainer approval, expected in Stage 04 or later.
- Decide which build becomes the first supported baseline at Stage 04
  integration.
- Document the update policy: a new build is a new tested configuration, not a
  silent replacement.

## Alternatives considered

- **Include provenance metadata in `EngineBuildId`:** rejected because unknown or
  corrected metadata would change the identifier of an unchanged executable and
  split its measurement history.
- **Include the executable path in build identity:** simple to implement, but
  makes identity depend on filesystem layout and breaks when a build is moved.
- **Include device-probe output in build identity:** treats one runtime
  observation as an immutable property of the executable and creates duplicate
  builds for the same artifact.
- **Track "latest" only:** convenient, but unreproducible and unsafe for support
  claims.
- **Trust the engine family's README:** fast, but wrong for specific builds,
  backends, and devices.
- **Infer from `--help` alone:** the flag may be accepted but unsupported by the
  selected backend, or may change semantics.
- **Use the version or release tag as build identity:** rejected; the same tag can
  map to multiple artifacts and metadata can be incomplete or corrected. The
  file hash identifies the executable.
- **Introduce a multi-file manifest fingerprint now:** deferred; no multi-file
  artifact identity is currently specified, and defining the schema without a
  concrete artifact would be speculative.

## References

- [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md)
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [ADR-0003](0003-engine-integration-strategy.md)
- [research notes](../research/2026-10-20-engine-landscape.md)
- [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
