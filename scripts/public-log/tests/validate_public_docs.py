#!/usr/bin/env python3
from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OPEN = ROOT / "docs" / "open-source"

REQUIRED = {
    "README.md",
    "CURRENT_STAGE.md",
    "ARCHITECTURE.md",
    "DEVELOPMENT_TIMELINE.md",
    "DECISIONS.md",
    "PROBLEMS_AND_SOLUTIONS.md",
    "CODEX_FIELD_NOTES.md",
    "INTERACTION_LOG_POLICY.md",
    "BUILD_IN_PUBLIC.md",
    "SECURITY_AND_PRIVACY.md",
    "ROADMAP.md",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")


actual = {path.name for path in OPEN.glob("*.md")}
require(actual == REQUIRED, f"public document set differs: {sorted(actual ^ REQUIRED)}")
require((ROOT / "docs" / "devlog" / "2026-09-20.md").is_file(), "daily devlog missing")

stage = (OPEN / "CURRENT_STAGE.md").read_text(encoding="utf-8")
for heading in (
    "## Goal",
    "## Implemented",
    "## Designed",
    "## Verified",
    "## Known Problems",
    "## Technical Decisions",
    "## Lessons Learned",
    "## Next",
):
    require(heading in stage, f"CURRENT_STAGE missing {heading}")
require("M1.5" in stage and "Blocked before M2" in stage, "M1.5 gate status is unclear")
require("126 Rust tests" in stage and "31 TypeScript tests" in stage, "verified counts missing")

policy = (OPEN / "INTERACTION_LOG_POLICY.md").read_text(encoding="utf-8")
for field in (
    "Timestamp",
    "Stage",
    "User Goal",
    "What Codex inspected",
    "What Codex changed",
    "Files changed",
    "Problems discovered",
    "Solution",
    "Commands and tests executed",
    "Result",
    "Remaining risks",
    "Next recommended step",
    "Codex engineering note",
):
    require(field in policy, f"interaction policy missing {field}")

queue = ROOT / "social" / "x" / "queue" / "current-stage.md"
template = ROOT / "social" / "x" / "templates" / "STAGE_POST_TEMPLATE.md"
require(queue.is_file(), "manual X queue missing")
require(template.is_file(), "manual X template missing")
queue_text = queue.read_text(encoding="utf-8")
require("status: blocked_remote" in queue_text, "manual X queue must disclose remote blocker")
require("github_reference: pending" in queue_text, "manual X queue must not invent a GitHub URL")
thread_body = queue_text.split("\n## Screenshot suggestions", 1)[0]
markers = list(re.finditer(r"(?m)^Thread\s+(\d+)/(\d+)\s*$", thread_body))
require(3 <= len(markers) <= 6, "manual X queue must contain 3-6 posts")
for expected, marker in enumerate(markers, start=1):
    require(int(marker.group(1)) == expected, "manual X queue order is invalid")
    require(int(marker.group(2)) == len(markers), "manual X queue total is invalid")
    end = markers[expected].start() if expected < len(markers) else len(thread_body)
    post = thread_body[marker.end() : end].strip()
    require(0 < len(post) <= 280, f"manual X post {expected} length is {len(post)}")

link_pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
for document in [ROOT / "README.md", *sorted(OPEN.glob("*.md"))]:
    text = document.read_text(encoding="utf-8")
    for target in link_pattern.findall(text):
        if target.startswith(("http://", "https://", "#")):
            continue
        clean_target = target.split("#", 1)[0]
        if not clean_target:
            continue
        require((document.parent / clean_target).resolve().exists(), f"broken link in {document.name}: {target}")

raw_home_prefix = "/" + "Users/"
for document in [ROOT / "README.md", ROOT / "AGENTS.md", *ROOT.joinpath("docs").rglob("*.md")]:
    require(raw_home_prefix not in document.read_text(encoding="utf-8"), f"raw home path in {document.relative_to(ROOT)}")

subprocess.run(
    [
        str(ROOT / "scripts" / "public-log" / "sanitize-public-log.sh"),
        "--check",
        str(ROOT / "README.md"),
        str(ROOT / "AGENTS.md"),
        str(OPEN),
        str(ROOT / "docs" / "devlog"),
    ],
    cwd=ROOT,
    check=True,
)

print("PASS: public documents, links, status markers, and interaction schema are valid")
