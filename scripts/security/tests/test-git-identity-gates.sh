#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

git init -q -b main "$fixture"
mkdir -p "$fixture/scripts/security" "$fixture/scripts/git/hooks"
cp "$source_root/scripts/security/check-git-identity.sh" "$source_root/scripts/security/check-public-history-identity.sh" "$fixture/scripts/security/"
cp "$source_root/scripts/git/hooks/pre-push" "$fixture/scripts/git/hooks/"
cp "$source_root/scripts/git/install-hooks.sh" "$fixture/scripts/git/"
git -C "$fixture" config --local user.name Fixture
git -C "$fixture" config --local user.email fixture@users.noreply.github.com
(cd "$fixture" && scripts/security/check-git-identity.sh && git commit --allow-empty -qm good && scripts/security/check-public-history-identity.sh HEAD && scripts/git/install-hooks.sh)

good_sha="$(git -C "$fixture" rev-parse HEAD)"
printf 'refs/heads/main %s refs/heads/main %s\n' "$good_sha" "$good_sha" | (cd "$fixture" && .git/hooks/pre-push origin example) >/dev/null

git -C "$fixture" config --local user.email private@example.invalid
if (cd "$fixture" && scripts/security/check-git-identity.sh) >"$fixture/identity-output" 2>&1; then
  echo "FAIL: non-noreply local identity passed" >&2
  exit 1
fi
if rg -q 'private@example.invalid' "$fixture/identity-output"; then
  echo "FAIL: identity gate printed a rejected email" >&2
  exit 1
fi
git -C "$fixture" commit --allow-empty -qm bad
git -C "$fixture" config --local user.email fixture@users.noreply.github.com
bad_sha="$(git -C "$fixture" rev-parse HEAD)"
if (cd "$fixture" && scripts/security/check-public-history-identity.sh HEAD) >"$fixture/history-output" 2>&1; then
  echo "FAIL: non-noreply history passed" >&2
  exit 1
fi
if rg -q 'private@example.invalid' "$fixture/history-output" || ! rg -q 'NON_NOREPLY_IDENTITY' "$fixture/history-output"; then
  echo "FAIL: history gate output is unsafe or incomplete" >&2
  exit 1
fi
if printf 'refs/heads/main %s refs/heads/main %s\n' "$bad_sha" "$good_sha" | (cd "$fixture" && .git/hooks/pre-push origin example) >"$fixture/hook-output" 2>&1; then
  echo "FAIL: pre-push accepted non-noreply main history" >&2
  exit 1
fi
if rg -q 'private@example.invalid' "$fixture/hook-output"; then
  echo "FAIL: pre-push printed a rejected email" >&2
  exit 1
fi

echo "PASS: repository identity, public history, and pre-push gates"
