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

1. **Build identity** — the executable or build artifact and its immutable
   metadata, including its SHA-256.
2. **Installation record** — where a local copy was found, how it was discovered,
   and other installation-specific details.
3. **Capability observation** — a timestamped result produced by probing that
   build in a specific runtime context.

If a path or a device probe is folded into build identity, moving an executable
or probing the same executable on another device appears to create a different
build. That makes reproducibility and support claims depend on local filesystem
layout and on a single observation.

## Decision

1. Treat **engine build identity** as the immutable identity of the executable or
   build artifact. It includes the executable SHA-256 and available immutable
   metadata: engine family, upstream or fork version, release tag or source
   commit, artifact name, target platform, build configuration, and known bundled
   dependency identity.
2. Derive `EngineBuildId` from the executable hash and the immutable build
   metadata needed to distinguish artifacts. The executable path is not an input.
   Moving or copying an executable does not create a different build.
3. Do not include runtime probe output in `EngineBuildId`. Different device
   results do not change the executable's identity.
4. Store an **installation record** separately. It records the executable path,
   discovery method (configured path, `PATH`, user-specified path, package, or
   other local source), installation details, and observation time. An
   installation record may change without changing build identity.
5. Store **capability observations** separately and timestamp them. Each
   observation is tied to an `EngineBuildId` and records the runtime context:
   backend, selected device, driver/runtime version, relevant dependency
   evidence, the probe command or endpoint, the result, and the supporting
   evidence.
6. Model each capability result as `supported`, `unsupported`, or `unknown`. A
   failed, missing, or inconclusive probe yields `unknown`, never `supported`.
   An observation from one backend, device, or driver does not automatically
   apply to another.
7. Record the selected backend and device explicitly, including the device
   selector used when multiple Vulkan adapters exist.
8. Keep CPU, CUDA 12.x, CUDA 13.x, and Vulkan as separate configurations. Do not
   merge their results into a single backend row.
9. Record whether a CUDA result used native SASS or PTX JIT when that can be
   determined. Do not assume the prebuilt architecture list from the engine
   family.
10. For externally discovered builds, record a release tag or source commit when
    available. Do not block discovery on unavailable provenance. Preserve the
    executable hash and known metadata, and label unknown provenance explicitly.
    Never invent a release, commit, version, or build configuration.
11. Do not auto-update an engine build during a benchmark or silently substitute
    a different build.
12. Treat a fork as a distinct build profile even when it reuses the same adapter
    code.
13. **Do not choose or pin a specific engine version in Stage 00 or Stage 01.**
    Select the first baseline during Stage 04 integration, after verifying build
    identity, checksum, runtime dependencies, device visibility, launch behavior,
    and the relevant capability observations. Until then, support claims remain
    unknown rather than tied to an assumed version.

## Consequences

Positive:

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

- The data model needs distinct build, installation, and capability records and
  explicit links between them.
- Externally discovered builds may have incomplete provenance; unknown fields
  must remain visible rather than inferred.
- Capability observations accumulate over time and need retention and selection
  rules.
- More metadata must be collected and validated.

Follow-up:

- Define the build, installation, and capability-observation schemas in Stage 01
  without installing an engine.
- Capture probe fixtures from pinned official builds only when an engine is
  installed with separate maintainer approval, expected in Stage 04 or later.
- Decide which build becomes the first supported baseline at Stage 04
  integration.
- Document the update policy: a new build is a new tested configuration, not a
  silent replacement.

## Alternatives considered

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
- **Pin only the release tag:** helpful, but artifacts with the same tag can
  differ by backend, build options, or runtime libraries; the hash and runtime
  context are still needed.

## References

- [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md)
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [ADR-0003](0003-engine-integration-strategy.md)
- [research notes](../research/2026-10-20-engine-landscape.md)
- [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
