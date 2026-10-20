# ADR-0004: Separate estimates from measurements and preserve provenance

**Status:** Accepted
**Date:** 2026-10-20
**Deciders:** `r31nu11` (project maintainer)
**Related:** [ARCHITECTURE.md](../ARCHITECTURE.md), [BENCHMARKING.md](../BENCHMARKING.md)

## Context

GhostRAM's product value depends on users being able to trust its claims. There
are several ways to produce a plausible but false performance number:

- treating a formula's output as if it were observed;
- reporting one successful run and hiding failures;
- comparing runs made with different models, engine builds, devices, context
  sizes, or power states;
- silently allowing `llama-server`'s `--fit` to adjust context or offload;
- recording a throughput number without the engine build, model hash, or backend;
- overwriting a measurement with a newer prediction.

Documentation alone cannot prevent this. The data model and report format must
make the distinction structural.

## Decision

1. Use separate types for `Estimate<T>` and `Measurement<T>`. There is no implicit
   conversion between them.
2. Every `Estimate<T>` carries its value or range, method, assumptions,
   confidence, creation time, and calibration dataset when one exists.
3. Every `Measurement<T>` carries the observed value, unit, sampling method and
   interval, run ID, engine build, device, and timestamp.
4. Every measured value belongs to a run manifest containing hardware, engine,
   model, resolved configuration, workload, raw samples, failures, and exclusions.
5. Reports label every performance number as `estimated`, `measured`, or
   `unknown`. A measured value without a manifest is invalid.
6. Records are immutable after completion. A correction is a new record that links
   to the superseded run; it does not rewrite history.
7. Raw samples are retained alongside summaries. Markdown summaries are generated
   from or checked against JSON, not edited into disagreement.
8. The planner may use measurements for calibration only in a labeled calibration
   record. A prediction never overwrites a measurement.
9. The tuner may rank candidates using estimates, but only measured results can
   support a claim that one configuration is faster or populate a Pareto frontier.
10. No secrets, API keys, private prompts, or model contents are stored in run
    records.

## Consequences

Positive:

- Claims remain auditable and reproducible.
- Estimator error can be measured and improved over time.
- Failed runs and regressions remain visible.
- Schema evolution can be versioned instead of silently changing semantics.

Negative:

- More fields and validation work per run.
- Reports are longer and less convenient to produce.
- Immutability means storage grows; retention policy must be designed later.
- Users may be disappointed when a plausible configuration is labeled
  "unverified" instead of "supported".

Follow-up:

- Define the JSON schema and schema-version policy in Stage 01.
- Implement manifest validation before any benchmark report is accepted.
- Add tests that reject a summary without provenance.
- Decide retention and export policy before SQLite or long-term history lands.

## Alternatives considered

- **Plain `f64` fields with comments:** simple, but impossible to enforce or audit.
- **One performance struct with a `source` string:** easier, but encourages
  accidental promotion and inconsistent labels.
- **Trust the engine's reported timings as complete measurements:** insufficient;
  they omit client-observed TTFT, host memory, failures, and configuration
  provenance.
- **Mutable run records:** convenient for corrections, but destroys auditability.

## References

- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [BENCHMARKING.md](../BENCHMARKING.md)
- [reports/benchmarks/README.md](../../reports/benchmarks/README.md)
