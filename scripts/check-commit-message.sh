#!/usr/bin/env bash
# Validate one commit message file against the GhostRAM commit convention.
# Usage: scripts/check-commit-message.sh <path-to-message-file>
set -euo pipefail

usage() {
  printf 'usage: %s <path-to-message-file>\n' "${0##*/}" >&2
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

message_file=$1
if [[ ! -f "$message_file" ]]; then
  printf 'error: message file not found: %s\n' "$message_file" >&2
  exit 2
fi

# Remove CRLF, Git template comments, and trailing whitespace. Keep intentional
# blank lines inside the body.
message=$(
  tr -d '\r' < "$message_file" |
    sed -e 's/^#.*$//' -e 's/[[:space:]]*$//' |
    sed -e '/./,$!d'
)

if [[ -z "${message//[[:space:]]/}" ]]; then
  printf 'error: commit message is empty\n' >&2
  exit 1
fi

header=$(printf '%s\n' "$message" | head -n 1)
header_length=${#header}

if (( header_length > 100 )); then
  printf 'error: header is %d characters; maximum is 100\n' "$header_length" >&2
  exit 1
fi

header_re='^(feat|fix|perf|refactor|test|docs|build|ci|chore|revert)(\([a-z0-9][a-z0-9._/-]*\))?!?: [a-z0-9][^[:cntrl:]]*$'
if ! printf '%s\n' "$header" | grep -Eq "$header_re"; then
  printf 'error: invalid header: %s\n' "$header" >&2
  printf 'expected: <type>(<scope>): <lowercase imperative description>\n' >&2
  printf 'allowed types: feat fix perf refactor test docs build ci chore revert\n' >&2
  exit 1
fi

if [[ "$header" == *.* ]]; then
  printf 'error: subject must not end with a period\n' >&2
  exit 1
fi

# A blank line separates the header from the body.
second_line=$(printf '%s\n' "$message" | sed -n '2p')
if [[ -n "${second_line//[[:space:]]/}" ]]; then
  printf 'error: leave a blank line between the header and the body\n' >&2
  exit 1
fi

type=${header%%(*}
type=${type%%!*}
type=${type%%:*}

if [[ "$type" == "revert" ]]; then
  # Revert commits are allowed to be header-only. If a body is present, it is
  # still expected to use the standard sections.
  body_lines=$(printf '%s\n' "$message" | tail -n +3 | sed '/^[[:space:]]*$/d' | wc -l)
  if (( body_lines == 0 )); then
    printf 'ok: revert commit accepted (header only)\n'
    exit 0
  fi
fi

for section in Changes Reason Validation; do
  if ! printf '%s\n' "$message" | grep -Eq "^${section}:[[:space:]]*$"; then
    printf 'error: missing required section: %s:\n' "$section" >&2
    exit 1
  fi
done

counts=$(
  printf '%s\n' "$message" | awk '
    /^Changes:[[:space:]]*$/    { section = "Changes";    next }
    /^Reason:[[:space:]]*$/     { section = "Reason";     next }
    /^Validation:[[:space:]]*$/ { section = "Validation"; next }
    /^[A-Za-z][A-Za-z ]*:[[:space:]]*$/ { section = "";   next }
    /^- /                       { if (section != "") count[section]++ }
    END { printf "%d %d %d\n", count["Changes"], count["Reason"], count["Validation"] }
  '
)
read -r changes reason validation <<< "$counts"

if (( changes == 0 || reason == 0 || validation == 0 )); then
  printf 'error: each section needs at least one "- " bullet (Changes=%d, Reason=%d, Validation=%d)\n' \
    "$changes" "$reason" "$validation" >&2
  exit 1
fi

if [[ "$header" == *"!"* ]]; then
  if ! printf '%s\n' "$message" | grep -Eq '^BREAKING CHANGE: .+'; then
    printf 'error: breaking-change header requires a "BREAKING CHANGE: ..." footer\n' >&2
    exit 1
  fi
fi

printf 'ok: commit message accepted\n'
