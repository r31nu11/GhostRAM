# Releasing

**Status:** Stage 00 policy. No version has been tagged or released. The project
maintainer is `r31nu11`. The first tag and any GitHub Release require explicit
maintainer approval separate from stage acceptance and promotion.

## 1. Versioning

GhostRAM follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

- `MAJOR.MINOR.PATCH` for the application and public interfaces.
- While the project is pre-1.0, `0.MINOR.PATCH` is used. Breaking changes may
  land in a minor release, but they must be documented in the changelog and
  release notes.
- Pre-releases use identifiers such as `0.3.0-alpha.1`, `0.3.0-beta.2`, or
  `0.3.0-rc.1`. Build metadata may use `+<commit>` but is not used for ordering.
- The version in the workspace manifest is the single source of truth once a
  Rust workspace exists. Tags are derived from it; they do not define it.
- A release candidate is not a final release. It must be tested from the
  candidate artifact, not only from a development build.

Version changes follow the impact on users and stored data, not the size of the
diff. A schema change that makes old run records unreadable is breaking even if
the code change is small.

## 2. Branch and tag policy

- Development and intermediate commits integrate on `dev`.
- `main` receives only maintainer-accepted, working/stable checkpoints.
- Promotion from `dev` to `main` requires maintainer acceptance and applicable
  validation. It is not automatic.
- A release is tagged from `main` at a reviewed commit.
- Tags are annotated and signed where the release process supports it.
- A tag or GitHub Release requires **separate explicit maintainer approval** after
  promotion. Stage acceptance does not authorize either.
- Pushes are not automatic.
- A documentation-only stage is not a stable application release.
- Do not move or delete a published tag. If a release is defective, publish a new
  version and document the problem.
- Do not tag an unreviewed commit or a dirty working tree.

No tag exists today.

## 3. Changelog

[CHANGELOG.md](../CHANGELOG.md) follows Keep a Changelog. Every release moves
entries from `Unreleased` into a dated version section. The changelog must state:

- user-visible features, fixes, and breaking changes;
- schema or data migrations;
- known limitations;
- security-relevant changes without disclosing an unpatched vulnerability.

Do not claim performance improvements without a benchmark report that meets
[BENCHMARKING.md](BENCHMARKING.md).

## 4. Release checklist

### Pre-release

- [ ] Stage acceptance criteria are met and the maintainer has reviewed the
      stage.
- [ ] `dev` is merged/promoted as approved; the release commit is on `main`.
- [ ] Working tree is clean and the commit is the intended one.
- [ ] Author and committer use the maintainer's confirmed identity; no automated
      or invented identity.
- [ ] The maintainer has approved publication separately from stage acceptance.
- [ ] Version is updated in the source of truth.
- [ ] Changelog is complete and dated.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `python3 scripts/check_docs.py` passes.
- [ ] Commit messages in the release range pass the range check.
- [ ] Install, quick-start, uninstall, and troubleshooting instructions match the
      artifact.
- [ ] No model weights, secrets, or unlicensed binaries are included.

### Artifact

- [ ] Build from the tagged commit with a recorded toolchain and build command.
- [ ] Produce the artifact, checksums, and third-party notices.
- [ ] Record the artifact size and supported platform.
- [ ] Verify the artifact on a clean environment and on the primary laptop where
      applicable.
- [ ] Run the documented installation and uninstall flow from the artifact.
- [ ] Preserve the build log and dependency lockfile.

### Publication (maintainer `r31nu11` only)

- [ ] The maintainer approves the version, notes, artifacts, publication channel,
      tag, and GitHub Release separately from stage acceptance and promotion.
- [ ] Create the annotated tag and release.
- [ ] Upload artifacts and checksums to the approved channel.
- [ ] Verify the published files match the local hashes.
- [ ] Announce only what was actually tested.

### Post-release

- [ ] Update `PROJECT_STATUS.md` and `ROADMAP.md`.
- [ ] Record the release in the changelog.
- [ ] Monitor for installation failures and regressions.
- [ ] If a defect requires a fix, prepare a patch release; do not silently replace
      the artifact.

## 5. What a release must not do

- Treat stage acceptance or a merge to `main` as permission to tag or publish.
- Publish a tag or GitHub Release without separate maintainer approval.
- Describe an untested platform as supported.
- Bundle an engine or model without a license review and explicit notices.
- Hide a failed test, a known crash, or a missing backend.
- Claim a speedup from an estimate or a single unverified run.
- Require root or modify drivers as part of installation.
- Download large assets silently during installation or first run.

## 6. Compatibility and support

The first release will state:

- supported OS and architecture;
- required or optional GPU backends;
- engine builds known to work;
- data/schema compatibility;
- known limitations and unsupported configurations.

Support is based on evidence from the release checklist, not on the engine
family's general documentation. A backend that works for upstream llama.cpp on
another GPU is not automatically supported here.

## 7. Current status

| Item | State |
| --- | --- |
| Version tagged | None. |
| GitHub release | None. |
| Artifact | None. |
| Checksums | None. |
| Changelog release section | None; only `Unreleased` entries for Stage 00, the accepted Stage 01 diagnostic implementation, the accepted Stage 02 model catalog, and the Stage 03 memory planner pending acceptance. |
| Publication | The Stage 00/01/02/03 development history was published to `origin/dev`. The initial Stage 03 hand-off was verified at `f2cff44abcbf5dadf421b12087c36d6631fd7829`; a documentation-only consistency correction is fast-forwarded and verified at hand-off. This is not approval for a tag, GitHub Release, `main` promotion, or artifact. No version or artifact exists. |
