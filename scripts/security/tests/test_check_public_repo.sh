#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
gate="$repo_root/scripts/security/check-public-repo.sh"
sanitizer="$repo_root/scripts/public-log/sanitize-public-log.sh"
fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

new_repo() {
  local name=$1
  local email=$2
  local repo="$fixture_root/$name"
  mkdir -p "$repo"
  git -C "$repo" init -q -b main
  git -C "$repo" config user.name "Public Gate Test"
  git -C "$repo" config user.email "$email"
  printf '%s\n' '# Safe repository' > "$repo/README.md"
  git -C "$repo" add README.md
  git -C "$repo" commit -q -m "safe fixture"
  printf '%s\n' "$repo"
}

[ -x "$gate" ] || fail "security gate is missing or not executable"

safe_repo=$(new_repo safe history-scan@example.invalid)
ORDINCONN_PUBLIC_REPO="$safe_repo" ORDINCONN_PUBLIC_SANITIZER="$sanitizer" "$gate" >/dev/null

private_repo=$(new_repo private-email private-person@example.com)
if output=$(ORDINCONN_PUBLIC_REPO="$private_repo" ORDINCONN_PUBLIC_SANITIZER="$sanitizer" "$gate" 2>&1); then
  fail "private commit email was not blocked"
fi
case "$output" in
  *private-commit-email*) ;;
  *) fail "private commit email did not return the expected category" ;;
esac
case "$output" in
  *private-person@example.com*) fail "private commit email value leaked" ;;
esac

echo "PASS: public repository gate scans worktree, history, paths, and commit metadata"
