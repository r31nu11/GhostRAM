#!/usr/bin/env bash
# Validate all non-merge commits in BASE..HEAD (exclusive of BASE).
# The root commit is skipped because it may predate the convention.
# Usage: scripts/check-commit-range.sh <base-sha> <head-sha>
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: %s <base-sha> <head-sha>\n' "${0##*/}" >&2
  exit 2
fi

base=$1
head=$2
repo_root=$(git rev-parse --show-toplevel)
checker="$repo_root/scripts/check-commit-message.sh"

if [[ ! -x "$checker" ]]; then
  printf 'error: checker is missing or not executable: %s\n' "$checker" >&2
  exit 2
fi

if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
  printf 'error: base is not a commit: %s\n' "$base" >&2
  exit 2
fi
if ! git rev-parse --verify --quiet "$head^{commit}" >/dev/null; then
  printf 'error: head is not a commit: %s\n' "$head" >&2
  exit 2
fi

mapfile -t commits < <(git rev-list --no-merges --reverse "$base..$head")
if [[ ${#commits[@]} -eq 0 ]]; then
  printf 'ok: no non-merge commits to check in %s..%s\n' "$base" "$head"
  exit 0
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

failed=0
for sha in "${commits[@]}"; do
  parent_count=$(git rev-list --parents -n 1 "$sha" | wc -w)
  if (( parent_count == 1 )); then
    printf 'skip: root commit %s\n' "${sha:0:12}"
    continue
  fi

  subject=$(git log -1 --format=%s "$sha")
  printf 'check: %s %s\n' "${sha:0:12}" "$subject"
  git log -1 --format=%B "$sha" > "$tmp"
  if ! "$checker" "$tmp"; then
    failed=$((failed + 1))
  fi
done

if (( failed > 0 )); then
  printf 'error: %d commit message(s) failed validation\n' "$failed" >&2
  exit 1
fi

printf 'ok: all checked commit messages passed\n'
