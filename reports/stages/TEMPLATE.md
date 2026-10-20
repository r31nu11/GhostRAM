# Stage report template

Use this template for every stage. Keep all 14 sections, even when a section is
"none" or "not applicable". State what was not done and why. Do not paste a
previous stage's claims without re-verifying them.

---

# Stage NN — Title

**Date:** YYYY-MM-DD
**Maintainer:** `r31nu11` (project maintainer)
**Branch:** `branch-name`
**Status:** In progress | Complete, pending acceptance | Accepted | Blocked

## 1. Stage identifier and title

State the stage number, exact title from the roadmap, and the review status.

## 2. Objectives and acceptance criteria

List the objectives and each acceptance criterion. Mark each as met, partially
met, not met, or not applicable, with evidence. A criterion is not met because the
code exists; it is met when the stated check was observed.

## 3. Summary of completed and incomplete work

Describe what was delivered and what was deferred. Include the reason for each
deferral.

## 4. Architectural decisions and rationale

List ADRs added, changed, or superseded. Summarize the decision and link to the
record. If no architectural decision was made, say so explicitly.

## 5. Files added, changed, or removed

Provide a reviewable table or list. Group by purpose. Include generated artifacts
only if they are intentionally committed.

## 6. Dependencies added or modified

List runtime, build, and development dependencies with versions and reason. If no
dependency was added, state that. Include toolchain changes and lockfile changes.

## 7. Validation commands and observed results

Record exact commands and the observed result. Do not write "passed" without the
command and output. Include failures and fixes.

```text
$ command
observed output
```

## 8. Hardware tests and measurements

Record machine, OS/kernel, driver, engine build, model hash, exact command,
observed result, and whether it was reproduced. If no hardware test was run, say
so and explain why. Do not report an estimate as a measurement.

## 9. Tests that could not be performed

List each test, the prerequisite that was missing, and the risk of leaving it
untested. Include environment limitations such as no GPU, no model, or no
approval.

## 10. Known bugs, limitations, and unresolved questions

List defects, incomplete behavior, uncertainty, and decisions requiring
maintainer direction. Distinguish a known limitation from an unverified
hypothesis.

## 11. Current branch and commit hash

Record the branch, the content commit, and the status/report commit if different.
State whether the commits are local or pushed. Never claim a push that did not
happen.

## 12. Git working-tree status

State the observed `git status` result. A clean tree at completion is expected; if
it is not clean, explain every remaining path.

## 13. Documentation updated

List the documents updated, including `PROJECT_STATUS.md`, `ROADMAP.md`, the
changelog, ADRs, and user-facing docs. Note documents that should have been
updated but were not.

## 14. Recommended next stage

Name the next stage, its prerequisites, and the maintainer decisions or approvals
required before it starts. Link to the roadmap entry.

---

## Completion checklist

- [ ] Acceptance criteria checked with evidence.
- [ ] Validation commands and observed results recorded.
- [ ] `PROJECT_STATUS.md` and `ROADMAP.md` updated.
- [ ] Changelog updated when user-visible behavior changed.
- [ ] ADRs added or superseded as needed.
- [ ] Working tree clean or remaining paths explained.
- [ ] Maintainer acceptance recorded.
- [ ] No push, tag, release, install, or system change without approval.
