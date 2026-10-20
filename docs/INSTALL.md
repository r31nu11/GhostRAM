# Installation

**Status: contract only. GhostRAM is not installable yet.** There is no release,
no package, and no binary. Do not run installation commands from this document;
none are provided because they would be false.

This page defines what "installed" will mean. The final, executable instructions
will be written and tested in Stage 08.

## 1. What will be required for ordinary users

The installation contract for a supported release is:

- **No Rust toolchain, Cargo, compiler, or build system.** Users will not be asked
  to compile GhostRAM.
- A supported 64-bit Linux distribution and desktop session. The exact supported
  matrix is not decided.
- Enough disk space for the application and its runtime dependencies. The exact
  figure will be measured from the artifact.
- An inference engine, or a documented way for GhostRAM to locate one. Whether the
  application bundles or downloads an engine is an open decision; it will not
  happen silently.
- At least one GGUF model supplied by the user, unless a future feature offers an
  explicit, checksummed download flow.
- A GPU driver and runtime appropriate to the chosen backend. A CUDA toolkit is
  not required for a Vulkan or CPU installation. A CUDA build may need a compatible
  runtime library; the packaging decision is pending.

The application will run as the normal user. It will not require root, a
system-wide service, or a driver change.

## 2. Planned installation channels

Candidates under evaluation:

- a versioned tarball with checksums;
- a distribution package;
- AppImage or Flatpak;
- a repository-provided installer.

No channel is selected. The choice must account for Vulkan/CUDA runtime
dependencies, desktop integration, updates, and uninstall behavior. It will be
recorded in an ADR before Stage 08.

## 3. What installation must not do

A GhostRAM installation must not:

- download model weights without explicit consent and a visible source/checksum;
- install GPU drivers or modify driver settings;
- change the CPU governor, clocks, or system-wide performance settings;
- require the user to disable security features or run as root;
- overwrite existing models, engine builds, or user configuration;
- claim that a backend works before a capability probe on that machine;
- bundle an engine under a license that forbids redistribution without notices.

## 4. Verification (future)

A future release must publish:

- the artifact and its SHA-256 checksum;
- the version and supported platform;
- third-party notices and licenses;
- a signature or an equivalent authenticity mechanism if the release process
  supports one;
- a short verification procedure using standard tools.

A checksum proves integrity only if it comes from a trusted channel. The project
will not describe an unsigned download as "verified".

## 5. Current state

| Item | State |
| --- | --- |
| GhostRAM binary | Does not exist. |
| Release artifact | None. |
| Package repository | None. |
| Supported distribution list | Not decided. |
| Engine bundled | No. |
| Model bundled | No. |
| Installer | Not written or tested. |

If you want to work on the source repository, see
[CONTRIBUTING.md](CONTRIBUTING.md). That is a development workflow, not a user
installation, and it currently produces no application.

## 6. Related documents

- [Quick start](QUICK_START.md) — planned first-run flow, not yet available.
- [Troubleshooting](TROUBLESHOOTING.md) — current limitations and diagnostics.
- [Uninstall](UNINSTALL.md) — planned data locations and removal rules.
- [Releasing](RELEASING.md) — release policy and checklist.
