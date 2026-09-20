#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
sync_script="$source_root/scripts/public-log/sync-public-devlog.sh"
sanitizer="$source_root/scripts/public-log/sanitize-public-log.sh"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

new_repo() {
  local name=$1
  local repo="$test_root/$name"
  mkdir -p "$repo/docs/open-source" "$repo/docs/devlog" "$repo/social/x/queue" "$repo/scripts/public-log" "$repo/src"
  git -C "$repo" init -q -b test-sync
  git -C "$repo" config user.name "Public Sync Test"
  git -C "$repo" config user.email "public-sync@example.invalid"
  printf '%s\n' '# Public' > "$repo/docs/open-source/README.md"
  printf '%s\n' '# Day' > "$repo/docs/devlog/2026-09-20.md"
  printf '%s\n' 'fn main() {}' > "$repo/src/product.rs"
  cp "$sanitizer" "$repo/scripts/public-log/sanitize-public-log.sh"
  chmod +x "$repo/scripts/public-log/sanitize-public-log.sh"
  git -C "$repo" add .
  git -C "$repo" commit -q -m initial
  printf '%s\n' "$repo"
}

add_remote() {
  local repo=$1
  local name=$2
  local remote="$test_root/$name.git"
  git init -q --bare "$remote"
  git -C "$repo" remote add origin "$remote"
  git -C "$repo" push -q -u origin test-sync
  printf '%s\n' "$remote"
}

run_sync() {
  local repo=$1
  ORDINCONN_PUBLIC_SYNC_REPO="$repo" \
  ORDINCONN_PUBLIC_SYNC_LOG="$test_root/sync.log" \
  "$sync_script"
}

[ -x "$sync_script" ] || fail "sync script is missing or not executable"

missing_repo=$(new_repo missing-remote)
printf '%s\n' 'new log' >> "$missing_repo/docs/devlog/2026-09-20.md"
missing_head=$(git -C "$missing_repo" rev-parse HEAD)
if output=$(run_sync "$missing_repo" 2>&1); then
  fail "missing remote unexpectedly succeeded"
fi
case "$output" in
  *GITHUB_REMOTE_REQUIRED*) ;;
  *) fail "missing remote did not report GITHUB_REMOTE_REQUIRED" ;;
esac
[ "$(git -C "$missing_repo" rev-parse HEAD)" = "$missing_head" ] || fail "missing remote created a commit"
git -C "$missing_repo" diff --quiet -- docs/devlog/2026-09-20.md && fail "missing remote discarded the local change"

clean_repo=$(new_repo no-changes)
add_remote "$clean_repo" clean-remote >/dev/null
clean_head=$(git -C "$clean_repo" rev-parse HEAD)
output=$(run_sync "$clean_repo")
case "$output" in
  *PUBLIC_DEVLOG_NO_CHANGES*) ;;
  *) fail "clean repository did not report no changes" ;;
esac
[ "$(git -C "$clean_repo" rev-parse HEAD)" = "$clean_head" ] || fail "no-op sync created a commit"

allowed_repo=$(new_repo allowed-change)
allowed_remote=$(add_remote "$allowed_repo" allowed-remote)
allowed_head=$(git -C "$allowed_repo" rev-parse HEAD)
printf '%s\n' 'verified update' >> "$allowed_repo/docs/devlog/2026-09-20.md"
run_sync "$allowed_repo" >/dev/null
new_head=$(git -C "$allowed_repo" rev-parse HEAD)
[ "$new_head" != "$allowed_head" ] || fail "allowed change did not create a commit"
[ "$(git --git-dir="$allowed_remote" rev-parse refs/heads/test-sync)" = "$new_head" ] || fail "allowed commit was not pushed"

staged_repo=$(new_repo staged-product)
add_remote "$staged_repo" staged-remote >/dev/null
printf '%s\n' '// user product edit' >> "$staged_repo/src/product.rs"
git -C "$staged_repo" add src/product.rs
printf '%s\n' 'public update' >> "$staged_repo/docs/devlog/2026-09-20.md"
staged_head=$(git -C "$staged_repo" rev-parse HEAD)
if output=$(run_sync "$staged_repo" 2>&1); then
  fail "out-of-scope staged product code unexpectedly succeeded"
fi
case "$output" in
  *OUT_OF_SCOPE_STAGED_FILES*) ;;
  *) fail "out-of-scope staging was not reported" ;;
esac
[ "$(git -C "$staged_repo" rev-parse HEAD)" = "$staged_head" ] || fail "staged product case created a commit"
git -C "$staged_repo" diff --cached --quiet -- src/product.rs && fail "staged product edit was altered"

reject_repo=$(new_repo push-rejection)
reject_remote=$(add_remote "$reject_repo" reject-remote)
printf '%s\n' '#!/usr/bin/env bash' 'exit 1' > "$reject_remote/hooks/pre-receive"
chmod +x "$reject_remote/hooks/pre-receive"
reject_head=$(git -C "$reject_repo" rev-parse HEAD)
printf '%s\n' 'preserve after rejection' >> "$reject_repo/docs/devlog/2026-09-20.md"
if output=$(run_sync "$reject_repo" 2>&1); then
  fail "rejecting remote unexpectedly succeeded"
fi
case "$output" in
  *GITHUB_PUSH_FAILED*) ;;
  *) fail "push rejection did not report GITHUB_PUSH_FAILED" ;;
esac
local_after_reject=$(git -C "$reject_repo" rev-parse HEAD)
[ "$local_after_reject" != "$reject_head" ] || fail "push rejection did not preserve a local commit"
[ "$(git --git-dir="$reject_remote" rev-parse refs/heads/test-sync)" = "$reject_head" ] || fail "rejecting remote unexpectedly advanced"
git -C "$reject_repo" diff --quiet -- docs/devlog/2026-09-20.md || fail "push rejection left committed public content dirty"

echo "PASS: sync is allowlisted, no-op safe, and preserves failed pushes"
