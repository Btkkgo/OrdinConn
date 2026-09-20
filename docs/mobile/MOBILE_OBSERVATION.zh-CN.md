# Mobile Observation

[English](MOBILE_OBSERVATION.md) | [简体中文](MOBILE_OBSERVATION.zh-CN.md)

`MobileObservation` 是一次已授权 Mobile Observation 的规范化结果。它包含 Task/Session Identity、Application/Package/Activity、Semantic Screen State、Timestamp、Frame/UI-tree Hash、Source Locator、Visible Fact、Extracted Entity、可选 Author/Publication Time、Extraction Method/Confidence、Redaction、Privacy Class、Verification Status 与 Metadata。

Screenshot 不是 Market Fact，Agent Summary 也不是 Source Evidence。因此 `MobileObservation` 与 Evidence 保持分离。

Promotion Path：

`MobileObservation -> deduplication -> source validation -> freshness -> reliability -> Evidence`

M1 使用 `observation_only` Evidence State 持久化 Observation，不会自动晋升它们或创建 Mobile-backed Signal。

Element Reference 以每个 `MobileUISnapshot` 为范围，生成 `@e1`、`@e2` 等标识。任何新 Snapshot 都会使旧 Snapshot 的所有 Reference 失效。
