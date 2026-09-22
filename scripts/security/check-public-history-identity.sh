#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
ref="${1:-main}"
commit="$(git -C "$repo_root" rev-parse --verify --quiet --end-of-options "${ref}^{commit}")" || {
  echo "FAIL: Public history ref is not a commit." >&2
  exit 2
}

python3 - "$repo_root" "$commit" <<'PY'
import re
import subprocess
import sys

repo, commit = sys.argv[1:]
valid = re.compile(rb"[^\s@]+@users\.noreply\.github\.com", re.IGNORECASE)
output = subprocess.check_output(
    ["git", "-C", repo, "log", "--format=%H%x00%ae%x00%ce", commit]
)
failed = []
for record in output.splitlines():
    parts = record.split(b"\0")
    if len(parts) != 3 or not parts[0]:
        print("FAIL: Malformed public history identity record.", file=sys.stderr)
        raise SystemExit(2)
    sha, author, committer = parts
    if not (valid.fullmatch(author) and valid.fullmatch(committer)):
        failed.append(sha.decode("ascii"))
if failed:
    for sha in failed:
        print(f"{sha} NON_NOREPLY_IDENTITY", file=sys.stderr)
    raise SystemExit(1)
print("PASS: Public reachable history identity is GitHub noreply compliant.")
PY
