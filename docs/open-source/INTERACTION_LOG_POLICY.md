# Public Interaction Log Policy

[English](INTERACTION_LOG_POLICY.md) | [简体中文](INTERACTION_LOG_POLICY.zh-CN.md)

## Purpose

Meaningful engineering work should leave a concise, auditable public summary. The source is the completed work and its verification evidence, never the raw private conversation.

## Required summary

Each `Public Interaction Summary` contains:

- Timestamp
- Stage
- GitHub Issue
- User Goal
- What Codex inspected
- What Codex changed
- Files changed
- Problems discovered
- Solution
- Commands and tests executed
- Result
- Remaining risks
- Next recommended step
- Codex engineering note

## Transformation

`Private interaction -> Technical extraction -> Redaction -> Fact check -> Public summary -> DevLog`

The English summary is appended to `docs/devlog/YYYY-MM-DD.md` and the synchronized Chinese summary to `docs/devlog/YYYY-MM-DD.zh-CN.md` after an engineering-significant task. Typo checks, status questions, or other no-change interactions do not need an entry unless they discover a reusable issue.

## Never record

- Full prompts or conversation transcripts
- Credentials, authorization material, cookies, or sessions
- Private account or customer information
- Private contact details or exact personal addresses
- Machine usernames or raw home-directory paths
- Unpublished business material or restricted third-party assets
- Claims that did not pass their stated acceptance check

## Status discipline

Use `Implemented`, `Designed`, `Verified`, and `Blocked` as defined in the [open-development index](README.md). Include the exact command or environment behind a verification claim when it materially affects the result.
