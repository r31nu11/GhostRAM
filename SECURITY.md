# Security policy

GhostRAM is not yet released and has no supported versions. This file defines
the intended disclosure process so that it is in place before the first binary
is published.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting / security advisory feature for the
`r31nu11/GhostRAM` repository. Do not open a public issue for an unfixed
vulnerability.

Include:

- affected commit or version,
- operating system and driver details,
- a minimal reproduction or proof of concept,
- expected and observed behavior,
- whether the issue affects GhostRAM itself, an inference engine, a model file,
  or the host system.

## Scope

GhostRAM is a control plane around external inference engines. Vulnerabilities
in llama.cpp, PrismML's fork, GPU drivers, or model files should also be reported
to the relevant upstream project. GhostRAM-specific issues include unsafe process
launching, path handling, resource-limit bypass, credential exposure, and
untrusted GGUF parsing performed by GhostRAM code.

## Disclosure

Please allow maintainers time to investigate and prepare a fix before public
disclosure. This policy will be revised when the project starts publishing
releases.
