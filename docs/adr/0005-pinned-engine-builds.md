# ADR-0005: Pin and fingerprint engine builds; probe capabilities per build

**Status:** Proposed
**Owner direction (2026-10-20):** Accepted in principle; line-by-line review pending.
**Date:** 2026-10-20
**Deciders:** `r31nu11` (repository owner), project engineers
**Related:** [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md), [ADR-0003](0003-engine-integration-strategy.md)

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
matrix that says "CUDA is supported" does not tell the user whether their
particular build contains code for compute capability 6.1.

## Decision

1. Pin every tested engine build by release tag or commit, artifact name, and
   binary SHA-256.
2. Assign an `EngineBuildId` derived from executable path, size, version/build
   information, hash, and backend/device probe output.
3. Store the engine family separately from the build. Family documentation is not
   evidence that a specific build supports a feature.
4. Probe each build at runtime: version/build info, visible devices, relevant
   `--help` flags, health/properties endpoints, and model-load behavior.
5. Model capabilities as `supported`, `unsupported`, or `unknown`, with evidence,
   source, observation time, and build ID. A failed or missing probe yields
   `unknown`, never `supported`.
6. Record the selected backend and device explicitly, including the device
   selector used when multiple Vulkan adapters exist.
7. Keep CPU, CUDA 12.x, CUDA 13.x, and Vulkan as separate configurations. Do not
   merge their results into a single backend row.
8. Record whether a CUDA result used native SASS or PTX JIT when that can be
   determined. Do not assume the prebuilt architecture list from the family.
9. Do not auto-update an engine build during a benchmark or silently substitute a
   different build.
10. Treat a fork as a distinct build profile even when it reuses the same adapter
    code.
11. **Do not choose or pin a specific engine version in Stage 00/01.** Select the
    first baseline during Stage 04 integration, after verifying the build,
    checksum, runtime dependencies, device visibility, and launch behavior. Until
    then, support claims remain unknown rather than tied to an assumed version.

## Consequences

Positive:

- Results can be reproduced and compared.
- Users get evidence about their actual executable, not a generic claim.
- A backend regression can be attributed to a build or device.
- Multiple engine builds can coexist without corrupting history.

Negative:

- More metadata to collect and validate.
- Pinned builds become stale; the project needs an update policy.
- Probing adds startup time and requires parsers for version/device output.
- Some capabilities cannot be determined without loading a model, which may be
  expensive or unsafe.

Follow-up:

- Define the `EngineBuildId` and capability schema in Stage 01 without installing
  an engine.
- Capture fixtures from pinned official builds only when an engine is installed
  with separate owner authorization, expected in Stage 04 or later.
- Decide which build becomes the first supported baseline at Stage 04 integration.
- Document the update policy: a new build is a new tested configuration, not a
  silent replacement.

## Alternatives considered

- **Track "latest" only:** simple, but unreproducible and unsafe for support
  claims.
- **Trust the engine family's README:** fast, but wrong for specific builds,
  backends, and devices.
- **Infer from `--help` alone:** the flag may be accepted but unsupported by the
  selected backend, or may change semantics.
- **Pin only the release tag:** helpful, but two artifacts with the same tag can
  differ by backend, build options, or runtime libraries; the hash and device
  probe are still needed.

## References

- [ENGINE_SUPPORT.md](../ENGINE_SUPPORT.md)
- [ADR-0003](0003-engine-integration-strategy.md)
- [research notes](../research/2026-10-20-engine-landscape.md)
- [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
