#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
scanner="$repo_root/scripts/public-log/sanitize-public-log.sh"
fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

expect_blocked() {
  local label=$1
  local path=$2
  local secret=${3:-}
  local output
  if output=$("$scanner" --check "$path" 2>&1); then
    fail "$label was not blocked"
  fi
  case "$output" in
    *PUBLIC_LOG_SCAN_BLOCKED*) ;;
    *) fail "$label did not return a fail-closed diagnostic" ;;
  esac
  if [ -n "$secret" ] && printf '%s' "$output" | grep -Fq "$secret"; then
    fail "$label leaked the suspected value"
  fi
}

[ -x "$scanner" ] || fail "sanitizer is missing or not executable"

printf '%s\n' 'A verified build completed with no private data.' > "$fixture_dir/safe.md"
"$scanner" --check "$fixture_dir/safe.md"

private_header="-----BEGIN PRIVATE"" KEY-----"
printf '%s\n' "$private_header" > "$fixture_dir/private.txt"
expect_blocked private-key "$fixture_dir/private.txt"

credential_value="not-a-real-value-1234567890"
printf 'API_%s="%s"\n' 'KEY' "$credential_value" > "$fixture_dir/assignment.txt"
expect_blocked credential-assignment "$fixture_dir/assignment.txt" "$credential_value"

bearer_value="not-a-real-bearer-1234567890"
printf 'Authorization: Bearer %s\n' "$bearer_value" > "$fixture_dir/authorization.txt"
expect_blocked bearer "$fixture_dir/authorization.txt" "$bearer_value"

printf 'ghp_%040d\n' 0 > "$fixture_dir/github-token.txt"
expect_blocked github-token "$fixture_dir/github-token.txt"

printf 'xoxb-%040d\n' 0 > "$fixture_dir/slack-token.txt"
expect_blocked slack-token "$fixture_dir/slack-token.txt"

printf 'Cookie: auth=%s\n' "$credential_value" > "$fixture_dir/cookie.txt"
expect_blocked cookie "$fixture_dir/cookie.txt" "$credential_value"

printf 'SESSION=%s\n' "$credential_value" > "$fixture_dir/session.txt"
expect_blocked session "$fixture_dir/session.txt" "$credential_value"

printf 'PASSWORD=%s\n' "$credential_value" > "$fixture_dir/password.txt"
expect_blocked password "$fixture_dir/password.txt" "$credential_value"

mnemonic_value='alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima'
printf 'MNEMONIC="%s"\n' "$mnemonic_value" > "$fixture_dir/mnemonic.txt"
expect_blocked mnemonic "$fixture_dir/mnemonic.txt" "$mnemonic_value"

raw_home="/""Users/""alice/Documents/OrdinConn"
printf '%s\n' "$raw_home" > "$fixture_dir/path.txt"
expect_blocked home-path "$fixture_dir/path.txt" "$raw_home"

printf '%s\n' 'SAFE_EXAMPLE=true' > "$fixture_dir/.env"
expect_blocked env-file "$fixture_dir/.env"

redacted=$("$scanner" --redact "$fixture_dir/path.txt")
[ "$redacted" = '~/Documents/OrdinConn' ] || fail "home path was not redacted"

mkdir "$fixture_dir/nested"
cp "$fixture_dir/safe.md" "$fixture_dir/nested/safe.md"
"$scanner" --check "$fixture_dir/nested"

echo "PASS: sanitizer blocks sensitive material and redacts home paths"
