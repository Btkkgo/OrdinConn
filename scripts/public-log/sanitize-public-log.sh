#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: sanitize-public-log.sh --check PATH... | --redact FILE" >&2
  exit 2
fi

mode=$1
shift

exec python3 - "$mode" "$@" <<'PY'
from __future__ import annotations

import os
import re
import sys
from pathlib import Path

mode = sys.argv[1]
inputs = [Path(value) for value in sys.argv[2:]]
excluded_dirs = {".git", ".superpowers", "node_modules", "target", "dist", "coverage"}

home_pattern = re.compile("/" + r"Users/[^/\s]+/")
private_key_pattern = re.compile(r"-----BEGIN (?:[A-Z0-9]+ )?PRIVATE" + r" KEY-----")
token_pattern = re.compile(
    r"(?:ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|"
    r"sk-[A-Za-z0-9_-]{20,}|xox[bp]-[A-Za-z0-9-]{20,})"
)
bearer_pattern = re.compile(r"(?im)^\s*Authorization\s*:\s*Bearer\s+\S+")
header_pattern = re.compile(r"(?im)^\s*(?:Cookie|Set-Cookie|Session)\s*:\s*\S+")
assignment_pattern = re.compile(
    r"(?im)^\s*(?:export\s+)?"
    r"(?:API_KEY|ACCESS_TOKEN|REFRESH_TOKEN|SECRET|PASSWORD|COOKIE|SESSION|"
    r"GITHUB_TOKEN|X_TOKEN|OPENAI_API_KEY|ANTHROPIC_API_KEY|GEMINI_API_KEY)"
    r"\s*[:=]\s*(?P<value>[^\r\n]+)"
)
mnemonic_pattern = re.compile(
    r"(?im)^\s*(?:MNEMONIC|SEED_PHRASE|RECOVERY_PHRASE)\s*[:=]\s*"
    r"[\"']?(?:[a-z]+\s+){11,23}[a-z]+[\"']?\s*$"
)


def iter_files(paths: list[Path]):
    for path in paths:
        if path.is_symlink():
            continue
        if path.is_file():
            yield path
            continue
        if not path.exists():
            yield path
            continue
        for root, dirs, files in os.walk(path, followlinks=False):
            dirs[:] = [name for name in dirs if name not in excluded_dirs]
            for name in files:
                candidate = Path(root) / name
                if not candidate.is_symlink():
                    yield candidate


def display(path: Path) -> str:
    try:
        return os.path.relpath(path, Path.cwd())
    except ValueError:
        return path.name


def read_text(path: Path) -> str | None:
    data = path.read_bytes()
    if b"\x00" in data[:8192]:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def meaningful_assignment(match: re.Match[str]) -> bool:
    value = match.group("value").strip().strip("\"'")
    normalized = value.lower()
    if not value:
        return False
    if value.startswith("${") or value.startswith("<"):
        return False
    return normalized not in {
        "redacted",
        "example",
        "example-only",
        "changeme",
        "not-configured",
        "unset",
    }


def categories(path: Path, text: str) -> list[str]:
    findings: list[str] = []
    name = path.name
    if (name == ".env" or name.startswith(".env.")) and name != ".env.example":
        findings.append("environment-file")
    checks = (
        ("private-key", private_key_pattern),
        ("token-shape", token_pattern),
        ("authorization-header", bearer_pattern),
        ("cookie-or-session-header", header_pattern),
        ("mnemonic-assignment", mnemonic_pattern),
        ("raw-home-path", home_pattern),
    )
    for label, pattern in checks:
        if pattern.search(text):
            findings.append(label)
    if any(meaningful_assignment(match) for match in assignment_pattern.finditer(text)):
        findings.append("credential-assignment")
    return findings


if mode == "--redact":
    if len(inputs) != 1 or not inputs[0].is_file():
        print("--redact requires exactly one readable file", file=sys.stderr)
        raise SystemExit(2)
    text = read_text(inputs[0])
    if text is None:
        print("--redact accepts UTF-8 text only", file=sys.stderr)
        raise SystemExit(2)
    sys.stdout.write(home_pattern.sub("~/", text))
    raise SystemExit(0)

if mode != "--check":
    print(f"unsupported mode: {mode}", file=sys.stderr)
    raise SystemExit(2)

blocked = False
seen: set[Path] = set()
for path in iter_files(inputs):
    if path in seen:
        continue
    seen.add(path)
    if not path.exists():
        print(f"PUBLIC_LOG_SCAN_BLOCKED read-error {display(path)}", file=sys.stderr)
        blocked = True
        continue
    try:
        text = read_text(path)
    except OSError:
        print(f"PUBLIC_LOG_SCAN_BLOCKED read-error {display(path)}", file=sys.stderr)
        blocked = True
        continue
    if text is None:
        continue
    for label in categories(path, text):
        print(f"PUBLIC_LOG_SCAN_BLOCKED {label} {display(path)}", file=sys.stderr)
        blocked = True

raise SystemExit(1 if blocked else 0)
PY
