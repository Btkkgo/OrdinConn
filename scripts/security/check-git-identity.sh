#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
name="$(git -C "$repo_root" config --local --get user.name || true)"
email="$(git -C "$repo_root" config --local --get user.email || true)"

if [[ -z "$name" || ! "$email" =~ ^[^[:space:]@]+@users\.noreply\.github\.com$ ]]; then
  echo "FAIL: Git identity email is not GitHub noreply compliant." >&2
  exit 1
fi

echo "PASS: Repository-local Git identity is GitHub noreply compliant."
