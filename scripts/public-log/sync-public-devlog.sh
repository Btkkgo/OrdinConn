#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
repo_root="${ORDINCONN_PUBLIC_SYNC_REPO:-}"
if [ -z "$repo_root" ]; then
  repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"
fi
repo_root="$(cd "$repo_root" && pwd -P)"
sanitizer="${ORDINCONN_PUBLIC_SANITIZER:-$repo_root/scripts/public-log/sanitize-public-log.sh}"
security_gate="${ORDINCONN_PUBLIC_SECURITY_GATE:-$repo_root/scripts/security/check-public-repo.sh}"
log_file="${ORDINCONN_PUBLIC_SYNC_LOG:-$HOME/Library/Logs/OrdinConn/github-sync.log}"
lock_key="$(printf '%s' "$repo_root" | cksum | awk '{print $1}')"
lock_dir="${TMPDIR:-/tmp}/ordinconn-public-devlog-sync-$lock_key.lock"
index_backup=""
index_path=""
restore_index=0
snapshot_dir=""

mkdir -p "$(dirname "$log_file")"

log_status() {
  local status=$1
  printf '%s %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$status" >> "$log_file"
}

fail() {
  local status=$1
  local code=${2:-1}
  log_status "$status"
  echo "$status" >&2
  exit "$code"
}

cleanup() {
  if [ "$restore_index" = "1" ] && [ -n "$index_backup" ] && [ -f "$index_backup" ]; then
    cp -p "$index_backup" "$index_path"
  fi
  if [ -n "$index_backup" ] && [ -f "$index_backup" ]; then
    rm -f "$index_backup"
  fi
  case "$snapshot_dir" in
    "${TMPDIR:-/tmp}"/ordinconn-public-index.*) rm -rf -- "$snapshot_dir" ;;
  esac
  rmdir "$lock_dir" 2>/dev/null || true
}

if ! mkdir "$lock_dir" 2>/dev/null; then
  fail "PUBLIC_DEVLOG_SYNC_ALREADY_RUNNING" 2
fi
trap cleanup EXIT

git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1 || fail "GIT_REPOSITORY_REQUIRED" 2
origin_url="$(git -C "$repo_root" remote get-url origin 2>/dev/null || true)"
[ -n "$origin_url" ] || fail "GITHUB_REMOTE_REQUIRED" 3
expected_remote="${ORDINCONN_PUBLIC_EXPECTED_REMOTE:-}"
is_official_remote() {
  local candidate=$1
  if [ -n "$expected_remote" ]; then
    [ "$candidate" = "$expected_remote" ]
    return
  fi
  case "$candidate" in
    git@github.com:Btkkgo/OrdinConn.git|https://github.com/Btkkgo/OrdinConn|https://github.com/Btkkgo/OrdinConn.git) return 0 ;;
    *) return 1 ;;
  esac
}
is_official_remote "$origin_url" || fail "OFFICIAL_GITHUB_REMOTE_REQUIRED" 3
push_urls="$(git -C "$repo_root" remote get-url --push --all origin 2>/dev/null || true)"
[ -n "$push_urls" ] || fail "OFFICIAL_GITHUB_REMOTE_REQUIRED" 3
while IFS= read -r push_url; do
  [ -n "$push_url" ] || continue
  is_official_remote "$push_url" || fail "OFFICIAL_GITHUB_REMOTE_REQUIRED" 3
done <<< "$push_urls"
[ -x "$sanitizer" ] || fail "PUBLIC_LOG_SANITIZER_REQUIRED" 2
[ -x "$security_gate" ] || fail "PUBLIC_REPOSITORY_SECURITY_GATE_REQUIRED" 2

branch="$(git -C "$repo_root" branch --show-current)"
[ -n "$branch" ] || fail "PUBLIC_DEVLOG_BRANCH_REQUIRED" 2

is_allowed_path() {
  case "$1" in
    docs/*|social/x/drafts/*|.github/ISSUE_TEMPLATE/*|.github/PULL_REQUEST_TEMPLATE.md) return 0 ;;
    *) return 1 ;;
  esac
}

if ! git -C "$repo_root" fetch --quiet origin "$branch"; then
  fail "GITHUB_FETCH_FAILED" 7
fi
remote_ref="refs/remotes/origin/$branch"
git -C "$repo_root" rev-parse --verify "$remote_ref" >/dev/null 2>&1 || fail "GITHUB_REMOTE_BRANCH_REQUIRED" 3
git -C "$repo_root" merge-base --is-ancestor "$remote_ref" HEAD || fail "GITHUB_REMOTE_AHEAD_OR_DIVERGED" 4

unpushed_out_of_scope=""
while IFS= read -r path; do
  [ -z "$path" ] && continue
  if ! is_allowed_path "$path"; then
    unpushed_out_of_scope="$unpushed_out_of_scope${unpushed_out_of_scope:+,}$path"
  fi
done < <(
  while IFS= read -r commit_id; do
    git -C "$repo_root" diff-tree -m --no-commit-id --name-only -r --no-renames "$commit_id"
  done < <(git -C "$repo_root" rev-list --reverse "$remote_ref..HEAD")
)
[ -z "$unpushed_out_of_scope" ] || fail "UNPUSHED_PRODUCT_COMMITS" 4

out_of_scope=""
while IFS= read -r path; do
  [ -z "$path" ] && continue
  if ! is_allowed_path "$path"; then
    out_of_scope="$out_of_scope${out_of_scope:+,}$path"
  fi
done < <(git -C "$repo_root" diff --cached --name-only)
[ -z "$out_of_scope" ] || fail "OUT_OF_SCOPE_STAGED_FILES" 4

if ! ORDINCONN_PUBLIC_REPO="$repo_root" ORDINCONN_PUBLIC_SANITIZER="$sanitizer" "$security_gate"; then
  fail "PUBLIC_LOG_SECRET_SCAN_FAILED" 5
fi

if ! git -C "$repo_root" diff --check; then
  fail "PUBLIC_LOG_DIFF_CHECK_FAILED" 5
fi

allowed_paths=()
for path in docs social/x/drafts .github/ISSUE_TEMPLATE .github/PULL_REQUEST_TEMPLATE.md; do
  if [ -e "$repo_root/$path" ]; then
    allowed_paths+=("$path")
  fi
done

git_dir="$(git -C "$repo_root" rev-parse --absolute-git-dir)"
index_path="$git_dir/index"
index_backup="$(mktemp "$git_dir/public-sync-index.XXXXXX")"
cp -p "$index_path" "$index_backup"
restore_index=1

if [ "${#allowed_paths[@]}" -gt 0 ]; then
  git -C "$repo_root" add -A -- "${allowed_paths[@]}"
fi

while IFS= read -r path; do
  [ -z "$path" ] && continue
  is_allowed_path "$path" || fail "OUT_OF_SCOPE_STAGED_FILES" 4
done < <(git -C "$repo_root" diff --cached --name-only)

if git -C "$repo_root" diff --cached --quiet; then
  restore_index=0
  log_status "PUBLIC_DEVLOG_NO_CHANGES"
  echo "PUBLIC_DEVLOG_NO_CHANGES"
  exit 0
fi

if ! git -C "$repo_root" diff --cached --check; then
  fail "PUBLIC_LOG_STAGED_DIFF_CHECK_FAILED" 5
fi

snapshot_dir="$(mktemp -d "${TMPDIR:-/tmp}/ordinconn-public-index.XXXXXX")"
if ! git -C "$repo_root" checkout-index --all --prefix="$snapshot_dir/"; then
  fail "PUBLIC_LOG_STAGED_SNAPSHOT_FAILED" 5
fi
if ! "$sanitizer" --check "$snapshot_dir"; then
  fail "PUBLIC_LOG_STAGED_SECRET_SCAN_FAILED" 5
fi
rm -rf -- "$snapshot_dir"
snapshot_dir=""

commit_message="docs(devlog): sync public development log $(date '+%Y-%m-%d %H:%M')"
if ! git -C "$repo_root" commit -m "$commit_message" >/dev/null; then
  fail "PUBLIC_DEVLOG_COMMIT_FAILED" 6
fi
restore_index=0

if ! git -C "$repo_root" push origin "$branch" >/dev/null 2>&1; then
  fail "GITHUB_PUSH_FAILED" 7
fi

log_status "PUBLIC_DEVLOG_SYNCED"
echo "PUBLIC_DEVLOG_SYNCED"
