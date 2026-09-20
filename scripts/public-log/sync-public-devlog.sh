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

if ! mkdir "$lock_dir" 2>/dev/null; then
  fail "PUBLIC_DEVLOG_SYNC_ALREADY_RUNNING" 2
fi
trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT

git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1 || fail "GIT_REPOSITORY_REQUIRED" 2
origin_url="$(git -C "$repo_root" remote get-url origin 2>/dev/null || true)"
[ -n "$origin_url" ] || fail "GITHUB_REMOTE_REQUIRED" 3
expected_remote="${ORDINCONN_PUBLIC_EXPECTED_REMOTE:-}"
if [ -n "$expected_remote" ]; then
  [ "$origin_url" = "$expected_remote" ] || fail "OFFICIAL_GITHUB_REMOTE_REQUIRED" 3
else
  case "$origin_url" in
    git@github.com:Btkkgo/OrdinConn.git|https://github.com/Btkkgo/OrdinConn|https://github.com/Btkkgo/OrdinConn.git) ;;
    *) fail "OFFICIAL_GITHUB_REMOTE_REQUIRED" 3 ;;
  esac
fi
[ -x "$sanitizer" ] || fail "PUBLIC_LOG_SANITIZER_REQUIRED" 2
[ -x "$security_gate" ] || fail "PUBLIC_REPOSITORY_SECURITY_GATE_REQUIRED" 2

branch="$(git -C "$repo_root" branch --show-current)"
[ -n "$branch" ] || fail "PUBLIC_DEVLOG_BRANCH_REQUIRED" 2

is_allowed_path() {
  case "$1" in
    docs/*|social/x/drafts/*|.github/*) return 0 ;;
    *) return 1 ;;
  esac
}

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
for path in docs social/x/drafts .github; do
  if [ -e "$repo_root/$path" ]; then
    allowed_paths+=("$path")
  fi
done

if [ "${#allowed_paths[@]}" -gt 0 ]; then
  git -C "$repo_root" add -A -- "${allowed_paths[@]}"
fi

while IFS= read -r path; do
  [ -z "$path" ] && continue
  is_allowed_path "$path" || fail "OUT_OF_SCOPE_STAGED_FILES" 4
done < <(git -C "$repo_root" diff --cached --name-only)

if git -C "$repo_root" diff --cached --quiet; then
  log_status "PUBLIC_DEVLOG_NO_CHANGES"
  echo "PUBLIC_DEVLOG_NO_CHANGES"
  exit 0
fi

if ! git -C "$repo_root" diff --cached --check; then
  fail "PUBLIC_LOG_STAGED_DIFF_CHECK_FAILED" 5
fi

commit_message="docs(devlog): sync public development log $(date '+%Y-%m-%d %H:%M')"
if ! git -C "$repo_root" commit -m "$commit_message" >/dev/null; then
  fail "PUBLIC_DEVLOG_COMMIT_FAILED" 6
fi

if ! git -C "$repo_root" push origin "$branch" >/dev/null 2>&1; then
  fail "GITHUB_PUSH_FAILED" 7
fi

log_status "PUBLIC_DEVLOG_SYNCED"
echo "PUBLIC_DEVLOG_SYNCED"
