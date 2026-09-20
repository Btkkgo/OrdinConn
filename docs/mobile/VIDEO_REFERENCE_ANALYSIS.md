# Video Reference Analysis

[English](VIDEO_REFERENCE_ANALYSIS.md) | [简体中文](VIDEO_REFERENCE_ANALYSIS.zh-CN.md)

The supplied 108.6-second reference video demonstrates a browser-hosted iOS Simulator, a live indicator, device selection, a Tools panel, direct pointer interaction, appearance changes, location/camera controls, and browser annotation that can be sent back to an Agent conversation.

The video shows an Event Log count and an AX Tree toggle but does not open either surface. It does not demonstrate AX node content, the transport protocol, Agent-originated Tap/Swipe/Type traces, structured post-action verification, or a complete observe-decide-act-verify loop. These remain reference behaviors rather than verified implementation details.

The visible annotation inspector reports browser DOM properties such as `div`, pixel dimensions, color, and font. OrdinConn therefore does not treat the reference annotation as proof of native element inspection. Its inspector links a selected visual region to a current UIAutomator/Accessibility node and labels visual-only fallback explicitly.

The reusable design ideas are co-location of Agent context and live device state, semantic inspection, auditable controls, and low-friction annotation. OrdinConn does not copy the simulator-management UI because its purpose is financial intelligence collection and evidence formation.
