#!/usr/bin/env bash
# Mirrors upstream master into `master` and rebases `espaciofuturo` (our patches) onto it.
# FORK.md "Sync". Used by .github/workflows/ef-sync.yml and by hand:
#   bash scripts/ef-sync.sh            # rebase locally, print the result, push nothing
#   bash scripts/ef-sync.sh --push     # also push master and espaciofuturo (force-with-lease)
# Exit codes: 0 synced (or nothing to do), 2 rebase conflict (files in $CONFLICTS_FILE).
set -euo pipefail

UPSTREAM_URL="${UPSTREAM_URL:-https://github.com/kane50613/takumi.git}"
CONFLICTS_FILE="${CONFLICTS_FILE:-ef-conflicts.txt}"
push=false
[ "${1:-}" = "--push" ] && push=true

git remote get-url upstream >/dev/null 2>&1 || git remote add upstream "$UPSTREAM_URL"
git fetch --quiet upstream master
git fetch --quiet origin master espaciofuturo

upstream=$(git rev-parse upstream/master)
old_tip=$(git rev-parse origin/espaciofuturo)
old_base=$(git merge-base origin/espaciofuturo upstream/master)

if ! git merge-base --is-ancestor origin/master upstream/master; then
  echo "::error::origin/master is not an ancestor of upstream/master; master must stay a pure mirror"
  exit 1
fi

if [ "$old_base" = "$upstream" ]; then
  echo "espaciofuturo already sits on upstream master ${upstream:0:10}; nothing to do"
  echo "changed=false" >> "${GITHUB_OUTPUT:-/dev/null}"
  exit 0
fi

echo "upstream master ${old_base:0:10} -> ${upstream:0:10} ($(git rev-list --count "$old_base..$upstream") commits)"
echo "patches: $(git rev-list --count "$old_base..$old_tip")"

git checkout --quiet -B espaciofuturo "$old_tip"
if ! git rebase --quiet "$upstream"; then
  git diff --name-only --diff-filter=U | tee "$CONFLICTS_FILE"
  echo "stopped at: $(git log -1 --format='%h %s' REBASE_HEAD 2>/dev/null || true)" | tee -a "$CONFLICTS_FILE"
  git rebase --abort
  exit 2
fi

echo "changed=true" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "sha=$(git rev-parse HEAD)" >> "${GITHUB_OUTPUT:-/dev/null}"
git log --oneline "$upstream..HEAD"

if $push; then
  git push origin "$upstream:refs/heads/master"
  git push --force-with-lease="espaciofuturo:$old_tip" origin espaciofuturo
fi
