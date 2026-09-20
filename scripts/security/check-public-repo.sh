#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
repo_root="${ORDINCONN_PUBLIC_REPO:-$(git -C "$script_dir" rev-parse --show-toplevel)}"
repo_root="$(cd "$repo_root" && pwd -P)"
sanitizer="${ORDINCONN_PUBLIC_SANITIZER:-$repo_root/scripts/public-log/sanitize-public-log.sh}"
python_bin="${ORDINCONN_PUBLIC_PYTHON:-python3}"

[ -x "$sanitizer" ] || { echo "SECURITY_GATE_FAILED sanitizer-required" >&2; exit 2; }
git -C "$repo_root" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
  echo "SECURITY_GATE_FAILED git-repository-required" >&2
  exit 2
}

if ! "$sanitizer" --check "$repo_root"; then
  echo "SECURITY_GATE_FAILED working-tree" >&2
  exit 1
fi

if ! "$sanitizer" --check-history "$repo_root"; then
  echo "SECURITY_GATE_FAILED git-history" >&2
  exit 1
fi

if ! "$python_bin" - "$repo_root" <<'PY'
from __future__ import annotations

import subprocess
import sys
from pathlib import PurePosixPath

repo = sys.argv[1]

tracked = subprocess.run(
    ["git", "-C", repo, "ls-files", "-z"],
    check=True,
    capture_output=True,
).stdout.decode("utf-8").split("\0")

blocked_dirs = {"node_modules", "target", "dist", "build", ".next", "coverage"}
blocked_paths: list[str] = []
for value in tracked:
    if not value:
        continue
    path = PurePosixPath(value)
    if any(part in blocked_dirs for part in path.parts):
        blocked_paths.append(value)
    if path.name == ".env" or (path.name.startswith(".env.") and path.name != ".env.example"):
        blocked_paths.append(value)

if blocked_paths:
    for value in sorted(set(blocked_paths)):
        print(f"SECURITY_GATE_FAILED prohibited-tracked-path {value}", file=sys.stderr)
    raise SystemExit(1)

emails = subprocess.run(
    ["git", "-C", repo, "log", "--all", "--format=%ae%n%ce"],
    check=True,
    capture_output=True,
    text=True,
).stdout.splitlines()

allowed_suffixes = ("@users.noreply.github.com", "@example.invalid", "@local")
if any(email and not email.lower().endswith(allowed_suffixes) for email in emails):
    print("SECURITY_GATE_FAILED private-commit-email", file=sys.stderr)
    raise SystemExit(1)
PY
then
  exit 1
fi

echo "SECURITY_GATE_PASSED"
