# Video Reference Analysis

[English](VIDEO_REFERENCE_ANALYSIS.md) | [简体中文](VIDEO_REFERENCE_ANALYSIS.zh-CN.md)

用户提供的 108.6 秒 Reference Video 展示 Browser-hosted iOS Simulator、Live Indicator、Device Selection、Tools Panel、直接 Pointer Interaction、Appearance Change、Location/Camera Control，以及可以发回 Agent Conversation 的 Browser Annotation。

Video 显示 Event Log Count 与 AX Tree Toggle，但没有打开它们。它没有展示 AX Node Content、Transport Protocol、Agent-originated Tap/Swipe/Type Trace、Structured Post-action Verification 或完整 Observe-decide-act-verify Loop。这些仍是 Reference Behavior，而不是已验证实现细节。

可见 Annotation Inspector 报告 `div`、Pixel Dimension、Color、Font 等 Browser DOM Property。因此 OrdinConn 不把 Reference Annotation 当作 Native Element Inspection 的证据。自身 Inspector 会把选定 Visual Region 关联到当前 UIAutomator/Accessibility Node，并明确标记 Visual-only Fallback。

可复用设计包括 Agent Context 与 Live Device State 共置、Semantic Inspection、Auditable Control 与低摩擦 Annotation。OrdinConn 不复制 Simulator-management UI，因为其目标是 Financial Intelligence Collection 与 Evidence Formation。
