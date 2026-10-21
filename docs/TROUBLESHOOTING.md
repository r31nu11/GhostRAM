# Troubleshooting

**Status:** Stage 03 source-only diagnostics. `ghostram doctor`,
`ghostram models scan`, and `ghostram models plan` exist, but there is no release or
installer. This page records the diagnostic commands, known environment
constraints, and behavior that later stages must explain.

## 1. First check: is this a GhostRAM problem?

If you did not build GhostRAM from source, any failure you see belongs to another
tool, the driver, the engine, or the operating system. GhostRAM did not install
itself and has no uninstaller because it is not installed.

Check the [project status](PROJECT_STATUS.md) before assuming a release or feature
exists.

## 2. GhostRAM doctor diagnostics

From a source checkout:

```sh
cargo run -p ghostram-cli -- doctor
cargo run -p ghostram-cli -- doctor --json
```

The command is read-only. Exit code 0 means the report was generated, including
when optional categories are `partial` or `unavailable`. Exit code 2 means the
invocation was invalid. Exit code 1 means the report could not be serialized or
written to stdout.

Common observations and what they mean:

| Output | Meaning |
| --- | --- |
| `nvidia_gpu: unavailable (nvidia-smi was not found on PATH)` | No usable NVIDIA management tool was found. This does not prove that no GPU or driver exists. |
| `vulkan: unavailable (vulkaninfo was not found on PATH)` | The Vulkan inspection tool was not found. GhostRAM did not install it. |
| `storage: unavailable (df was not found on PATH)` | The read-only capacity probe could not run. Other categories may still be available. |
| `engines: available (no llama.cpp executables were found on PATH)` | Engine discovery completed and found no supported executable name. Installing an engine is not part of Stage 01. |
| `tools: partial (...)` | Tool discovery completed, but at least one found tool's bounded version probe failed or timed out. |
| `Overall: partial` | At least one category is incomplete or unavailable. Inspect the category reasons and the JSON report. |

For machine-readable output, validate that stdout is exactly one JSON object:

```sh
cargo run -q -p ghostram-cli -- doctor --json > /tmp/opencode/doctor.json
python3 -m json.tool /tmp/opencode/doctor.json > /dev/null
```

The schema is documented in [DOCTOR_SCHEMA.md](DOCTOR_SCHEMA.md). A `partial` or
`unavailable` result is an observation, not a claim that a backend is supported.

## 3. GhostRAM model scan diagnostics

From a source checkout, select a directory explicitly:

```sh
cargo run -p ghostram-cli -- models scan /path/to/models
cargo run -p ghostram-cli -- models scan /path/to/models --recursive
cargo run -p ghostram-cli -- models scan /path/to/models --json
cargo run -p ghostram-cli -- models scan /path/to/models --hash
```

The scanner is read-only and non-recursive by default. It does not follow
symlinks, does not search `$HOME` or a configured directory, and does not load or
modify model files. Without `--hash`, tensor payloads are not read.

Exit code 0 means the scan completed, including `partial` and `empty` results. A
`partial` scan is not a command failure: inspect each file record and the
`skipped`/`issues` fields. Exit code 2 means an invalid invocation. Exit code 1
means the root could not be scanned or output failed.

| Observation | Meaning and action |
| --- | --- |
| `state: empty` | No candidate `.gguf` files were found at this depth. Try a different selected path or `--recursive`. This is not an error. |
| `state: partial` | At least one candidate or traversal entry was not fully inspected. Read the per-file `parse_status` and `failure`, plus `skipped` and `issues`. |
| `parse_status: unsupported` | Unsupported GGUF version/endianness/metadata type, or a documented catalog limit was exceeded. The file may still be valid for another reader. |
| `parse_status: malformed` | Bad magic, duplicate names, invalid alignment/tensor descriptor, or truncation. Do not assume an engine will accept it. |
| `parse_status: unreadable` | Permission, I/O, or non-regular-file failure. Check access to the selected tree. |
| `parse_status: changed` | The file changed during inspection. Avoid concurrent writes and retry. Best-effort detection is not a lock or a race-free snapshot. |
| `hash.status: failed` | Hashing was requested but the complete file could not be read. No digest is reported. |
| `hash.status: changed` | The digest was discarded because the file changed or its read length differed. Retry after writes finish. |
| A symlinked model is missing | Symlinks are deliberately skipped, including a symlink supplied as the root. Select the real directory. |
| A non-UTF-8 filename is missing | Such candidate paths are skipped, counted, and make the scan `partial`; JSON display paths require UTF-8. |
| `--hash` takes a long time | It explicitly reads every complete candidate file, including tensor bytes. It is not an incremental or cached hash. |

A symlink root, missing root, non-directory root, unreadable root, or non-UTF-8
root is a command failure, not a partial scan. A path beginning with `-` needs the
`--` option terminator. The full field contract is in
[MODELS_SCHEMA.md](MODELS_SCHEMA.md); limits are in
[GGUF_CATALOG.md](GGUF_CATALOG.md).

## 4. GhostRAM memory plan diagnostics

The planner requires one explicit GGUF file:

```sh
cargo run -p ghostram-cli -- models plan /path/to/model.gguf
cargo run -p ghostram-cli -- models plan /path/to/model.gguf --json
```

It is read-only, does not hash the file, and does not scan the parent directory.
It observes host memory and NVIDIA device memory for this invocation. Exit code 0
means a plan envelope was produced, including a partial plan with unknown
quantities. Exit code 2 means the invocation or option value was invalid. Exit code
1 means the selected path could not be selected safely or output failed.

| Observation | Meaning and action |
| --- | --- |
| `parse_status: malformed` or `unsupported` | The file remains observable but weights and KV are unavailable. Select a supported GGUF v2/v3 little-endian file; do not assume an engine will accept it. |
| `insufficient_data` | At least one required observation, estimate, or device is unknown. Read `unknowns`, `warnings`, and the scenario reasons. This is not a fit result. |
| `estimated_components_exceed_budget` | The modeled weights and/or KV exceed an effective budget. Reduce context, sequences, or model size, or adjust policy reserves deliberately. Runtime overhead is still excluded. |
| `estimated_components_within_budget` | Only the modeled components are within budget. Launch success, engine support, backend workspace, and runtime overhead remain unverified. |
| `kv_cache_unavailable` / `unsupported_attention_layout` | The narrow uniform `llama` profile is not applicable, or a recognized special layout is present. No KV number is invented. |
| `device_weights_cpu_kv` is `insufficient_data` | No NVIDIA device was selected or observed. This does not prove that no GPU exists; check `ghostram doctor` and `nvidia-smi`. |
| An explicit `--device N` is missing | The requested index was not observed. Run `ghostram doctor` or `nvidia-smi -L` and use a reported index. |
| `--kv-cache q8_0` is rejected | Only F16 and F32 are modeled. Quantized KV is not a simple linear divisor and is deferred. |
| `context estimate` is huge or capped | It is a budget upper bound after the file-size proxy, not a recommendation or engine limit. Check declared context and warnings. |
| The file-size proxy looks too small | It is only the GGUF file size. It does not model tensor allocation, quantization overhead, alignment, mmap, or GPU layer placement. |
| A directory or symlink is rejected | Select one real regular file. The planner never selects automatically or follows a symlink. |

`--context` accepts 1 to 1,048,576 tokens and `--sequences` accepts 1 to 256.
Reserve options are non-negative MiB values. The full contract is in
[PLAN_SCHEMA.md](PLAN_SCHEMA.md); planner limits and evidence rules are in
[MEMORY_PLANNER.md](MEMORY_PLANNER.md).

## 5. Known constraints on the development laptop

These observations were made on 2026-10-20 and may change:

| Symptom | Likely cause | What is known |
| --- | --- | --- |
| CUDA 13 toolkit cannot build for Pascal | CUDA 13 removed offline compilation and library support for Pascal (compute capability 6.1). | Use a CUDA 12.x toolkit to build for Pascal, or use Vulkan. The distro `cuda` package is 13.4.x and is unsuitable as a Pascal build toolchain. A prebuilt labeled CUDA 13.x must be checked for actual device code and runtime libraries; the label alone proves neither compatibility nor incompatibility. |
| Vulkan runs on the Intel iGPU instead of the NVIDIA GPU | Both Intel UHD 630 and NVIDIA GTX 1050 Ti expose Vulkan devices. | Explicit NVIDIA device selection is required. Do not assume the first enumerated device is discrete. |
| The machine becomes sluggish or the OOM killer fires | 7.4 GiB RAM and 4 GiB VRAM are shared with the desktop; swap is zram. | Use conservative budgets, smaller contexts, and one engine at a time. A successful load is not proof of usability. |
| A prebuilt CUDA binary does not start | `libcudart`, cuBLAS, or a compatible runtime may be missing. | The official CUDA archive ships runtime assets separately; a toolkit alone is not the only option. This was not tested locally. |
| No model is listed | No regular, UTF-8-named `.gguf` file was found in the selected directory, or recursion was not requested. | GhostRAM does not download models or invent a default path. An empty scan is a successful `empty` result. |
| No engine is found | No llama.cpp, Ollama, or other engine was installed. | Engine installation is a future, explicit step. Stage 01 did not install one. |
| `cmake`, `ninja`, or `nvcc` is missing | The development toolchain is not installed. | Building an engine locally requires explicit maintainer approval and additional packages. End users will not need a compiler for GhostRAM. |

## 6. Safe diagnostics

The following commands are read-only and exist on a typical Linux system. They
diagnose the machine, not GhostRAM. Run only what is available; missing tools are
themselves useful information.

```sh
uname -a
cat /etc/os-release
lscpu
free -h
swapon --show
nvidia-smi
vulkaninfo --summary
df -h
```

For memory pressure:

```sh
cat /proc/pressure/memory
cat /proc/meminfo
```

For a running engine process, replace `<pid>` with the actual process ID:

```sh
cat /proc/<pid>/status
cat /proc/<pid>/smaps_rollup
```

Do not run commands as root to "fix" a GhostRAM issue. Do not change the driver,
governor, or clocks as a first response. Record the output and the exact engine
build before changing anything.

## 7. Interpreting common engine failures (future)

GhostRAM should eventually classify errors instead of dumping raw logs:

- **Engine not found:** no executable at the configured path or on `PATH`.
- **Unsupported build:** the executable exists but lacks the required backend or
  model format.
- **Model rejected:** invalid GGUF, unsupported architecture or quantization, or a
  missing shard.
- **Budget violation:** predicted memory exceeds the configured RAM/VRAM margin.
- **Startup timeout:** the health endpoint did not become ready in time.
- **Engine crash:** non-zero exit, signal, OOM kill, or driver reset.
- **Protocol error:** malformed or unexpected HTTP response.
- **Pressure abort:** the safety governor stopped the run because memory pressure
  or swap growth exceeded the threshold.

These categories are design requirements, not implemented behavior.

## 8. Reporting a problem

Once the project has a public issue tracker and a release, a useful report
includes:

- GhostRAM version and commit;
- OS, kernel, CPU, RAM, GPU, VRAM, and driver;
- engine executable, version, and build;
- model hash and quantization;
- exact command or configuration;
- full error text and relevant logs;
- whether the run was on AC power and what else was running.

Do not include API keys, tokens, private prompts, or model files. For security
vulnerabilities, follow [SECURITY.md](../SECURITY.md), not a public issue.

## 9. Current limitations

- There is no GhostRAM release or installer; only source-built diagnostic,
  model-scan, and memory-plan commands exist.
- The model scan is a point-in-time observation with no cache, persistent catalog,
  shard grouping, or engine cross-check.
- The planner uses synthetic-test evidence only; it has not compared its file-size
  proxy or KV formula with a real model, engine allocation, backend workspace, or
  runtime overhead.
- Plans are ephemeral, limited to two scenarios and one narrow KV profile, and
  cannot refuse or authorize a launch because no launch path exists.
- No ecosystem parser was audited and no real GGUF model was tested by the project.
- No persistent logs or automatic diagnostic bundle exist.
- No telemetry is collected.
- No inference, engine installation, or benchmark workflow exists to recover.
- No recovery action for a real inference run has been validated on this hardware.
- The secondary GTX 1070 machine has not been accessed.
