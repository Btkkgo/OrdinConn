#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
configured="$(git -C "$repo_root" config --local --get core.hooksPath || true)"
if [[ -n "$configured" ]]; then
  echo "FAIL: Existing core.hooksPath must be reviewed before installing hooks." >&2
  exit 1
fi

source_hook="$repo_root/scripts/git/hooks/pre-push"
common_dir="$(git -C "$repo_root" rev-parse --path-format=absolute --git-common-dir)"
target_hook="$common_dir/hooks/pre-push"
if [[ -e "$target_hook" ]] && ! cmp -s "$source_hook" "$target_hook"; then
  echo "FAIL: Existing pre-push hook must be reviewed before replacement." >&2
  exit 1
fi
mkdir -p "$(dirname "$target_hook")"
install -m 0755 "$source_hook" "$target_hook"
echo "PASS: Versioned pre-push hook installed."
