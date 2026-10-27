#!/usr/bin/env bash
# Prints the lines our patches change in upstream-owned files: every such line is a future rebase
# conflict. A file that exists at the upstream base is upstream's; a file the patches add is ours.
# FORK.md "Patch surface". Called from scripts/ef-sync.sh so EF Sync's log tracks the number.
#   bash scripts/ef-patch-surface.sh                 # upstream/master...HEAD
#   bash scripts/ef-patch-surface.sh <base> <tip>    # e.g. an older espaciofuturo
set -euo pipefail

tip="${2:-HEAD}"
base=$(git rev-parse "${1:-$(git merge-base upstream/master "$tip")}")
upstream_lines=0
own_lines=0
own_files=0

while IFS=$'\t' read -r added deleted path; do
  # Binary files (goldens) count neither way.
  [ "$added" = "-" ] && continue
  lines=$((added + deleted))
  if git cat-file -e "$base:$path" 2>/dev/null; then
    printf '%6d  %s\n' "$lines" "$path"
    upstream_lines=$((upstream_lines + lines))
  else
    own_lines=$((own_lines + lines))
    own_files=$((own_files + 1))
  fi
done < <(git diff --numstat "$base" "$tip")

echo "patch surface: $upstream_lines lines in upstream-owned files ($own_lines lines in $own_files files of our own) on ${base:0:10}"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "Patch surface: **$upstream_lines** lines in upstream-owned files ($own_lines in $own_files of our own)." >> "$GITHUB_STEP_SUMMARY"
fi
