# Changelog

All notable changes to GhostRAM will be documented in this file. The project
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

No GhostRAM version has been released or tagged. `Unreleased` describes the
documentation foundation only; it is not a runnable product.

## [Unreleased]

### Added

- Project charter, MVP definition, non-goals, success criteria, and governance.
- Architecture proposal with crate boundaries, domain-model rules, and safety
  principles.
- Dated engine research covering official llama.cpp, CUDA/Pascal constraints,
  Vulkan, and PrismML's llama.cpp fork, with sources and confidence levels.
- Engine compatibility matrix and initial integration strategy.
- Benchmark methodology covering TTFT, prompt processing, generation, memory,
  swap pressure, load time, failures, controls, and reporting.
- `dev`/`main` branch policy, Conventional Commits template, local commit hook,
  commit-range validation, and documentation checks.
- Semantic Versioning, release-candidate, and promotion checklists.
- User installation, quick start, troubleshooting, and uninstall documentation
  contracts, explicitly marked as not yet available.
- Stage report template, Stage 00 report, and benchmark report layout.
- Architecture decision records for workflow, crate decomposition, engine
  integration, measurement provenance, and build pinning.

### Changed

- Expanded `README.md` from a placeholder into the public project overview.
- Established `dev` as the integration branch; `main` remains the accepted
  checkpoint branch.

### Fixed

- Nothing yet; no application code exists.

### Removed

- Nothing.

### Security

- Added an initial security disclosure policy; no release support matrix exists
  yet.
