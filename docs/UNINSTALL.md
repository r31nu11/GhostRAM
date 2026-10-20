# Uninstall

**Status: not applicable yet.** GhostRAM has no installer, package, binary, or
release. Nothing in this document can be executed today.

This page defines the future uninstall contract so that installation does not
create hidden, undeletable state.

## 1. Planned removal behavior

A GhostRAM uninstall should remove:

- the application binary or package-managed files;
- application-owned configuration created by GhostRAM;
- caches and generated reports that GhostRAM owns;
- desktop entries, icons, and services installed by the package;
- temporary files and logs created by the application.

It should not remove, by default:

- user model files;
- model directories;
- engine installations that GhostRAM did not create;
- engine downloads or caches managed by another tool;
- user-created prompts, datasets, or benchmark exports stored outside
  GhostRAM-owned directories;
- shared system libraries or drivers.

The final paths and ownership rules will be decided with the packaging format in
Stage 08. Until then, this document deliberately does not list concrete paths or
commands that might delete the wrong files.

## 2. Data locations (to be finalized)

GhostRAM may use the XDG base-directory conventions on Linux:

| Kind | Planned location | Contains |
| --- | --- | --- |
| Configuration | `$XDG_CONFIG_HOME/ghostram` (fallback `~/.config/ghostram`) | User settings, model directories, budgets. |
| Data/history | `$XDG_DATA_HOME/ghostram` (fallback `~/.local/share/ghostram`) | Run records, measurements, model metadata cache. |
| Cache | `$XDG_CACHE_HOME/ghostram` (fallback `~/.cache/ghostram`) | Regenerable caches, temporary downloads with consent. |
| Logs | To be decided | Bounded engine and application logs. |

These are proposals. They must be implemented and tested before being documented
as actual locations. A future `ghostram doctor` or `ghostram paths` command should
print the effective paths rather than requiring users to guess.

## 3. Future uninstall flow

1. Stop any running GhostRAM-managed engine process.
2. Export or back up run history if the user wants to keep it.
3. Remove the package or application directory using the channel's documented
   method.
4. Remove GhostRAM-owned configuration and data only after showing the paths and
   asking for confirmation.
5. Leave models, engines, and user data untouched unless the user explicitly
   selects them.
6. Verify that no GhostRAM process or service remains.

No step above has been implemented or tested.

## 4. Current cleanup

If you cloned the repository for development, deleting the clone removes only the
checkout. It does not remove models, engines, or system packages, because Stage 00
installed none. Do not run recursive deletion commands against a directory that
contains your models or other work.

## 5. Related documents

- [Installation contract](INSTALL.md)
- [Troubleshooting](TROUBLESHOOTING.md)
- [Project status](PROJECT_STATUS.md)
