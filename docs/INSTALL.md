# Installation

**Status: contract plus source-development notes. GhostRAM is not installable
yet.** There is no release, package, or installer. A source-buildable `ghostram`
binary exists in the repository, but that is a development artifact, not a
supported installation.

This page defines what "installed" will mean. The final, executable end-user
instructions will be written and tested in Stage 08.

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
| GhostRAM source binary | Buildable with Cargo; `ghostram doctor` and `ghostram models scan` are implemented. |
| Release artifact | None. |
| Package repository | None. |
| Supported distribution list | Not decided. |
| Engine bundled | No. |
| Model bundled | No. |
| Installer | Not written or tested. |

If you want to work on the source repository, see
[CONTRIBUTING.md](CONTRIBUTING.md). That is a development workflow, not a user
installation. A developer can build and run the read-only source commands:

```sh
cargo build --workspace --locked
cargo run -p ghostram-cli -- doctor
cargo run -p ghostram-cli -- doctor --json
cargo run -p ghostram-cli -- models scan /path/to/models
```

These commands require a Rust toolchain and compile the workspace. They do not
install GhostRAM, create a package, install an engine, or download a model. The
model scanner reads only an explicitly selected directory and writes no catalog
state. See [GGUF_CATALOG.md](GGUF_CATALOG.md) for its boundaries.

## 6. Related documents

- [Quick start](QUICK_START.md) — source diagnostic check and planned first-run flow.
- [Troubleshooting](TROUBLESHOOTING.md) — current limitations and diagnostics.
- [Uninstall](UNINSTALL.md) — planned data locations and removal rules.
- [Releasing](RELEASING.md) — release policy and checklist.
