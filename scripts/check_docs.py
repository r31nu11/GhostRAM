#!/usr/bin/env python3
"""Lightweight documentation integrity checks for GhostRAM.

This is intentionally dependency-free: it runs with the Python standard library
and checks the properties that matter for the current documentation contract.
It is not a general-purpose Markdown linter.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

REQUIRED_FILES = [
    "README.md",
    "AGENTS.md",
    "CHANGELOG.md",
    "Cargo.toml",
    "Cargo.lock",
    "docs/PROJECT_CHARTER.md",
    "docs/PROJECT_STATUS.md",
    "docs/ROADMAP.md",
    "docs/ARCHITECTURE.md",
    "docs/DOCTOR_SCHEMA.md",
    "docs/TESTING.md",
    "docs/ENGINE_SUPPORT.md",
    "docs/CONTRIBUTING.md",
    "docs/INSTALL.md",
    "docs/QUICK_START.md",
    "docs/TROUBLESHOOTING.md",
    "docs/UNINSTALL.md",
    "docs/BENCHMARKING.md",
    "docs/RELEASING.md",
    "docs/adr/README.md",
    "reports/stages/stage-00-technical-reconnaissance.md",
    "reports/stages/stage-01-minimal-rust-workspace.md",
    "reports/stages/TEMPLATE.md",
    "reports/benchmarks/README.md",
]

REQUIRED_REPORT_SECTIONS = [
    "Stage identifier and title",
    "Objectives and acceptance criteria",
    "Summary of completed and incomplete work",
    "Architectural decisions and rationale",
    "Files added, changed, or removed",
    "Dependencies added or modified",
    "Validation commands and observed results",
    "Hardware tests and measurements",
    "Tests that could not be performed",
    "Known bugs, limitations, and unresolved questions",
    "Current branch and commit hash",
    "Git working-tree status",
    "Documentation updated",
    "Recommended next stage",
]

LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def markdown_files() -> list[Path]:
    files: list[Path] = []
    for path in ROOT.rglob("*.md"):
        parts = set(path.relative_to(ROOT).parts)
        if parts & {".git", "target", "node_modules"}:
            continue
        files.append(path)
    return sorted(files)


def check_required_files(errors: list[str]) -> None:
    for relative in REQUIRED_FILES:
        if not (ROOT / relative).is_file():
            errors.append(f"missing required file: {relative}")


def check_links(path: Path, errors: list[str]) -> int:
    text = path.read_text(encoding="utf-8")
    checked = 0
    in_fence = False
    fence_marker = ""
    for line_number, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            marker = stripped[:3]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
                fence_marker = ""
            continue
        if in_fence:
            continue
        # Ignore inline code spans so examples do not become link checks.
        line = re.sub(r"`[^`]*`", "", line)
        for match in LINK_RE.finditer(line):
            raw_target = match.group(1).strip()
            if raw_target.startswith("<") and raw_target.endswith(">"):
                raw_target = raw_target[1:-1]
            target = raw_target.split()[0]
            if target.startswith(("http://", "https://", "mailto:", "tel:", "data:", "#")):
                continue
            target = unquote(target.split("#", 1)[0])
            if not target:
                continue
            candidate = (
                ROOT / target.lstrip("/")
                if target.startswith("/")
                else path.parent / target
            )
            checked += 1
            if not candidate.exists():
                errors.append(
                    f"{path.relative_to(ROOT)}:{line_number}: "
                    f"broken relative link: {raw_target}"
                )
    return checked


def check_adrs(errors: list[str]) -> None:
    adr_dir = ROOT / "docs" / "adr"
    for path in sorted(adr_dir.glob("[0-9][0-9][0-9][0-9]-*.md")):
        text = path.read_text(encoding="utf-8")
        for required in ("Status:", "Date:", "## Context", "## Decision", "## Consequences"):
            if required not in text:
                errors.append(
                    f"{path.relative_to(ROOT)}: missing ADR field/section: {required}"
                )


def check_stage_reports(errors: list[str]) -> None:
    report_paths = sorted(ROOT.glob("reports/stages/stage-[0-9][0-9][0-9][0-9]-*.md"))
    for path in report_paths:
        text = path.read_text(encoding="utf-8")
        for heading in REQUIRED_REPORT_SECTIONS:
            if heading not in text:
                errors.append(
                    f"{path.relative_to(ROOT)}: missing stage report section: {heading}"
                )


def main() -> int:
    errors: list[str] = []
    check_required_files(errors)

    files = markdown_files()
    checked_links = 0
    for path in files:
        checked_links += check_links(path, errors)

    check_adrs(errors)
    check_stage_reports(errors)

    if errors:
        print(f"documentation check failed with {len(errors)} error(s):", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(
        f"ok: {len(files)} Markdown files, {checked_links} relative links, "
        f"{len(REQUIRED_FILES)} required files"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
