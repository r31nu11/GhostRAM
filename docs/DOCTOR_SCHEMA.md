# Diagnostic report schema

**Status:** Implemented in Stage 01. The command is source-only; no GhostRAM
release artifact exists.
**Schema identifier:** `ghostram.doctor/1`.
**Implementation:** `crates/ghostram-core/src/diagnostic.rs` and
`crates/ghostram-probe/src/report.rs`.

`ghostram doctor` is a read-only inspection command. It reports what it observed
and what it could not observe. It does not install engines or models, launch
inference, benchmark hardware, or change system settings. An executable or device
being visible is not a claim that GhostRAM supports it.

## 1. Envelope

`ghostram doctor --json` writes one JSON object to stdout. It writes nothing else
to stdout. Operational errors, when they occur, go to stderr and use a non-zero
exit code.

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | Report contract identifier. Currently `ghostram.doctor/1`. |
| `application_version` | string | Version of the `ghostram` binary that generated the report. Development builds report `0.1.0-dev`. |
| `generated_at` | string | UTC RFC 3339 generation time with whole-second precision, for example `2026-10-20T19:43:09Z`. |
| `report` | object | Diagnostic payload described below. |

The envelope is serialized from a typed Rust structure. Consumers must reject an
unknown major schema identifier rather than guessing at field meanings.

## 2. Report and category states

`report.overall_state` is derived from the ten category states:

| State | Meaning |
| --- | --- |
| `available` | The category completed and its data is usable. A valid empty result, such as "no engine found", is still `available`. |
| `partial` | Some data was obtained, but at least one field or probe is missing or inconclusive. `reason` explains why. |
| `unavailable` | No usable data was obtained for that category. `reason` explains why. |

`report.categories` contains one object per category: `os`, `cpu`, `memory`,
`swap`, `zram`, `storage`, `nvidia_gpu`, `vulkan`, `tools`, and `engines`. Each
object has a `state` and may have a `reason`.

Missing optional hardware or tools therefore do not make the JSON malformed or
the command fail. They produce `unavailable` or `partial` categories and a
`partial` overall state when other categories succeeded. A category marked
`available` is not a claim that a backend, engine, or configuration is supported.

## 3. System data

`report.system` contains only observed data. Optional fields are omitted from JSON
when unknown; they are never replaced with guessed values. Byte quantities are
plain JSON integers representing exact bytes, not strings or rounded megabytes.

| Field | Meaning |
| --- | --- |
| `os` | Distribution name, version, identifier, kernel release, and architecture. Version may be absent when the distribution uses rolling identifiers. |
| `cpu` | Vendor, model, architecture, logical CPU count, and distinct physical cores when detectable. |
| `memory` | `total_bytes`, `available_bytes`, and `free_bytes` from `/proc/meminfo`. |
| `swap` | Total, used, and free bytes, plus devices from `/proc/swaps`. |
| `zram` | Sysfs zram devices with uncompressed disk size and selected compression algorithm. An empty list means none were detected. |
| `storage` | Capacity and availability for `/`, read through `df -P -B1 /`. |
| `nvidia_gpus` | `nvidia-smi` GPU index, name, driver version, VRAM total/used, and compute capability. |
| `vulkan` | Vulkan instance version and physical devices from `vulkaninfo --summary`, including normalized device type, vendor ID, device ID, and driver text. |
| `tools` | Presence and bounded `--version` text for selected development, GPU, and inference tools on `PATH`. `path: null` means not found. |
| `engines` | Discovered `llama-server`, `llama-bench`, and `llama-cli` executables on `PATH`, with bounded version text and, for `llama-server`, `--list-devices` output. |

An empty `engines` array with state `available` means discovery completed and no
supported executable name was found. It does not mean GhostRAM installed or
verified an engine.

## 4. Probe bounds

- Filesystem probes read `/etc/os-release`, `/proc`, and `/sys` directly.
- `df`, `nvidia-smi`, `vulkaninfo`, and discovered llama.cpp executables are
  invoked with explicit executable paths and argument arrays. No shell is used.
- Version queries have a 2-3 second timeout; Vulkan summary has an 8 second
  timeout; llama.cpp device listing has a 5 second timeout.
- Captured stdout and stderr are bounded in memory. A timed-out child is killed
  and reaped.
- A timeout, non-zero exit, or unparseable output makes the relevant category
  `partial` or `unavailable`; it is never reported as supported.
- Only `llama-server`, `llama-bench`, and `llama-cli` are considered engine
  executables. No filesystem-wide executable search is performed.

## 5. Privacy

The report intentionally excludes hostnames, usernames as account metadata,
environment dumps, serial numbers, UUIDs, model contents, prompts, API keys, and
command history. It contains local executable paths and the root mount point,
which can reveal installation choices. Review a report before attaching it to a
public issue.

## 6. Example

This is a reduced real report from the development laptop on 2026-10-20. Some
repeated fields are omitted for length; this example is illustrative, not a
fixture used by tests.

```json
{
  "schema_version": "ghostram.doctor/1",
  "application_version": "0.1.0-dev",
  "generated_at": "2026-10-20T19:43:09Z",
  "report": {
    "overall_state": "available",
    "categories": {
      "os": { "state": "available" },
      "cpu": { "state": "available" },
      "memory": { "state": "available" },
      "swap": { "state": "available" },
      "zram": { "state": "available" },
      "storage": { "state": "available" },
      "nvidia_gpu": { "state": "available" },
      "vulkan": { "state": "available" },
      "tools": { "state": "available" },
      "engines": {
        "state": "available",
        "reason": "no llama.cpp executables were found on PATH"
      }
    },
    "system": {
      "os": {
        "name": "CachyOS",
        "id": "cachyos",
        "kernel": "7.2.11-1-cachyos",
        "architecture": "x86_64"
      },
      "cpu": {
        "vendor": "GenuineIntel",
        "model": "Intel(R) Core(TM) i7-8750H CPU @ 2.20GHz",
        "architecture": "x86_64",
        "logical_cpus": 12,
        "physical_cores": 6
      },
      "memory": {
        "total_bytes": 7989465088,
        "available_bytes": 3470852096,
        "free_bytes": 1058377728
      },
      "swap": {
        "total_bytes": 7989096448,
        "used_bytes": 56410112,
        "free_bytes": 7932686336,
        "devices": [
          {
            "name": "/dev/zram0",
            "kind": "partition",
            "size_bytes": 7989096448,
            "used_bytes": 56410112,
            "priority": 100
          }
        ]
      },
      "zram": {
        "devices": [
          {
            "name": "zram0",
            "disksize_bytes": 7989100544,
            "algorithm": "zstd"
          }
        ]
      },
      "storage": [
        {
          "filesystem": "/dev/sdb2",
          "mount_point": "/",
          "total_bytes": 124895825920,
          "used_bytes": 74713227264,
          "available_bytes": 43790991360
        }
      ],
      "nvidia_gpus": [
        {
          "index": 0,
          "name": "NVIDIA GeForce GTX 1050 Ti",
          "driver_version": "580.178.04",
          "memory_total_bytes": 4294967296,
          "memory_used_bytes": 3145728,
          "compute_capability": "6.1"
        }
      ],
      "vulkan": {
        "instance_version": "1.4.363",
        "devices": [
          {
            "index": 0,
            "name": "Intel(R) UHD Graphics 630 (CFL GT2)",
            "device_type": "integrated_gpu",
            "api_version": "1.4.354",
            "driver_version": "26.2.4",
            "driver_name": "Intel open-source Mesa driver",
            "driver_info": "Mesa 26.2.4-arch3.1",
            "vendor_id": "0x8086",
            "device_id": "0x3e9b"
          },
          {
            "index": 1,
            "name": "NVIDIA GeForce GTX 1050 Ti",
            "device_type": "discrete_gpu",
            "api_version": "1.4.312",
            "driver_version": "580.178.4.0",
            "driver_name": "NVIDIA",
            "driver_info": "580.178.04",
            "vendor_id": "0x10de",
            "device_id": "0x1c8c"
          }
        ]
      },
      "tools": [
        { "name": "cmake", "path": null },
        {
          "name": "nvidia-smi",
          "path": "/usr/bin/nvidia-smi",
          "version": "NVIDIA-SMI version  : 580.178.04"
        }
      ],
      "engines": []
    }
  }
}
```

## 7. Exit codes and output modes

| Invocation | stdout | Exit code |
| --- | --- | --- |
| `ghostram doctor` | Human-readable report | 0 when collection completes, even if categories are `partial` or `unavailable` |
| `ghostram doctor --json` | Exactly one JSON envelope and a trailing newline | 0 when collection completes |
| `ghostram --help`, `ghostram --version` | Help or version text | 0 |
| Invalid command or option | Nothing | 2 |
| Report serialization or stdout failure | Nothing reliable | 1 |

## 8. Compatibility rules

- Additive, optional fields may be added within `ghostram.doctor/1`.
- Renaming, removing, or changing the meaning/type of a field requires a new
  schema identifier such as `ghostram.doctor/2`.
- Consumers should ignore unknown fields and reject unknown schema identifiers.
- The report is a point-in-time observation. It is not a benchmark result and
  must not be used as evidence of inference performance.
