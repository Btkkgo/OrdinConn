#!/usr/bin/env python3
from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OPEN = ROOT / "docs" / "open-source"

PRIMARY_PUBLIC_DOCS = {
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

REQUIRED = PRIMARY_PUBLIC_DOCS | {
    name.removesuffix(".md") + ".zh-CN.md" for name in PRIMARY_PUBLIC_DOCS
}

CORE_PAIRS = (
    ("README.md", "README.zh-CN.md"),
    ("CHANGELOG.md", "CHANGELOG.zh-CN.md"),
    ("CONTRIBUTING.md", "CONTRIBUTING.zh-CN.md"),
    ("SECURITY.md", "SECURITY.zh-CN.md"),
    ("docs/CURRENT_STATUS.md", "docs/CURRENT_STATUS.zh-CN.md"),
    ("docs/ARCHITECTURE.md", "docs/ARCHITECTURE.zh-CN.md"),
    ("docs/architecture/OVERVIEW.md", "docs/architecture/OVERVIEW.zh-CN.md"),
    ("docs/roadmap/README.md", "docs/roadmap/README.zh-CN.md"),
    ("docs/PROBLEMS_AND_SOLUTIONS.md", "docs/PROBLEMS_AND_SOLUTIONS.zh-CN.md"),
    ("docs/codex/CODEX_FIELD_NOTES.md", "docs/codex/CODEX_FIELD_NOTES.zh-CN.md"),
    ("docs/decisions/README.md", "docs/decisions/README.zh-CN.md"),
    ("docs/devlog/2026-09-20.md", "docs/devlog/2026-09-20.zh-CN.md"),
    ("social/x/README.md", "social/x/README.zh-CN.md"),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")


actual = {path.name for path in OPEN.glob("*.md")}
require(actual == REQUIRED, f"public document set differs: {sorted(actual ^ REQUIRED)}")
for english_name, chinese_name in CORE_PAIRS:
    english = ROOT / english_name
    chinese = ROOT / chinese_name
    require(english.is_file(), f"English core document missing: {english_name}")
    require(chinese.is_file(), f"Chinese core document missing: {chinese_name}")
    require("English" in english.read_text(encoding="utf-8") and "简体中文" in english.read_text(encoding="utf-8"), f"language switch missing: {english_name}")
    require("English" in chinese.read_text(encoding="utf-8") and "简体中文" in chinese.read_text(encoding="utf-8"), f"language switch missing: {chinese_name}")

stage = (ROOT / "docs" / "CURRENT_STATUS.md").read_text(encoding="utf-8")
for heading in (
    "## Implemented",
    "## Verified",
    "## Partial",
    "## Blocked",
    "## Designed",
    "## Planned",
    "## Not Started",
    "## Next",
):
    require(heading in stage, f"CURRENT_STATUS missing {heading}")
require("- Gate: **M1.5 PASS**" in stage and "- Next phase: **M2 NOT STARTED**" in stage, "M1.5 gate status is unclear")
require("125 Rust tests" in stage and "31 TypeScript tests" in stage, "verified counts missing")

stage_zh = (ROOT / "docs" / "CURRENT_STATUS.zh-CN.md").read_text(encoding="utf-8")
require("- Gate：**M1.5 通过**" in stage_zh and "- 下一阶段：**M2 尚未开始**" in stage_zh, "Chinese M1.5 gate status is unclear")
require("125 个测试" in stage_zh and "31 个 TypeScript 测试" in stage_zh, "Chinese verified counts missing")

agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
for marker in ("## BUILD IN PUBLIC LANGUAGE POLICY", "Chinese by default", "English is the primary/default", "`## English`", "`## 中文参考`"):
    require(marker in agents, f"AGENTS language policy missing {marker}")

policy = (OPEN / "INTERACTION_LOG_POLICY.md").read_text(encoding="utf-8")
for field in (
    "Timestamp",
    "Stage",
    "GitHub Issue",
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

queue = ROOT / "social" / "x" / "drafts" / "0001-introducing-ordinconn.md"
template = ROOT / "social" / "x" / "templates" / "STAGE_POST_TEMPLATE.md"
require(queue.is_file(), "manual X draft missing")
require(template.is_file(), "manual X template missing")
queue_text = queue.read_text(encoding="utf-8")
require("DRAFT" in queue_text, "manual X draft must disclose draft state")
require("https://github.com/Btkkgo/OrdinConn" in queue_text, "manual X draft must include the verified repository URL")
require("https://github.com/Btkkgo/OrdinConn/issues/1" in queue_text, "manual X draft must include the stage Issue")
require("## English" in queue_text and "## 中文参考" in queue_text, "manual X draft must be bilingual")
thread_body = queue_text.split("### Suggested Thread", 1)[1].split("### Suggested Screenshots", 1)[0]
markers = list(re.finditer(r"(?m)^###\s+(\d+)/(\d+)\s*$", thread_body))
require(len(markers) == 5, "introductory X draft must contain 5 posts")
for expected, marker in enumerate(markers, start=1):
    require(int(marker.group(1)) == expected, "manual X queue order is invalid")
    require(int(marker.group(2)) == len(markers), "manual X queue total is invalid")
    end = markers[expected].start() if expected < len(markers) else len(thread_body)
    post = thread_body[marker.end() : end].strip()
    require(0 < len(post) <= 280, f"manual X post {expected} length is {len(post)}")

for draft in sorted(ROOT.joinpath("social", "x", "drafts").glob("*.md")):
    draft_text = draft.read_text(encoding="utf-8")
    require("## English" in draft_text and "## 中文参考" in draft_text, f"X draft is not bilingual: {draft.name}")

template_text = template.read_text(encoding="utf-8")
require("## English" in template_text and "## 中文参考" in template_text, "X draft template is not bilingual")

for issue_file in sorted(ROOT.joinpath("docs", "issues").glob("*.md")):
    issue_text = issue_file.read_text(encoding="utf-8")
    require("# English" in issue_text and "# 中文" in issue_text, f"Issue record is not bilingual: {issue_file.name}")
    for english_field, chinese_field in (
        ("## Goal", "## 目标"),
        ("## Current State", "## 当前状态"),
        ("## Acceptance Criteria", "## 验收标准"),
        ("## Result", "## 结果"),
        ("## Validation", "## 验证"),
    ):
        require(english_field in issue_text and chinese_field in issue_text, f"Issue record missing bilingual field in {issue_file.name}: {english_field}")

link_pattern = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
for document in [ROOT / "README.md", ROOT / "README.zh-CN.md", *sorted(ROOT.joinpath("docs").rglob("*.md"))]:
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
