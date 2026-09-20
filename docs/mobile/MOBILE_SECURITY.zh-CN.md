# Mobile Runtime Security

[English](MOBILE_SECURITY.md) | [简体中文](MOBILE_SECURITY.zh-CN.md)

## 默认 Policy

Application Access 默认为 Deny-all。Package 必须由用户放入 Allowlist 后，其 UI Content 才能被观察。Runtime 永远不采集 Password、OTP Value、Seed Phrase、Private Key、Payment Detail 或 Secure-field Content。

## 禁止行为

- Automatic Login、Password/OTP Reading、Cookie Theft、CAPTCHA Bypass
- Private-message Collection、Rooting、APK Modification、TLS Interception 或 Private API
- Posting、Commenting、Messaging、Liking、Following 或 Account Mutation
- Real Order、Transfer、Deposit、Withdrawal、Wallet Signing、Private Key 或 Seed Phrase

与 Password、OTP、Wallet、Banking、Payment、Buy、Sell、Long、Short、Place Order、Withdraw、Transfer 或 Sign Flow 相关的 Node/Screen 会被 Redact 或 Block。Event Payload 只包含 Identifier、Hash、Status 与 Count，不包含完整私密内容。

Frame 默认不持久化。只有已脱敏 Semantic Snapshot 与 Structured Observation 进入 SQLite。Research Task 具有明确 Duration、Step、Scroll、Page、Observation 与 Model-call Budget。

## Environment 与 AVD Safety

Environment Discovery 只读。OrdinConn 可以在解析准确 Name 后启动现有 AVD，但不会创建、编辑、Reset、删除或自动终止 AVD，也不会安装 Android Studio、Command-line Tool、System Image、Application 或 Permission。

M1.5 Gate Fail Closed。Tool 缺失或 Device Offline 会阻止所有下游 Frame、UI Tree、Observation、IPC 与 Shutdown Acceptance Claim。Malformed Sensitive/Financial Node 也会使整个 UI-tree Parse 失败；只有 Malformed Non-sensitive Platform Node 可以被丢弃。Test Fixture 只证明 Parser Behavior，不作为 Real-device Evidence。

## 真实 Redaction 验收

M1.5 Validation 使用一个含 Password Input 与非个人临时 Test Value 的本地 Temporary Application。UIAutomator 将该 Node 标记为 Password，且没有暴露 Plaintext。OrdinConn 记录 Redaction，将 Snapshot 标记为 `SensitiveFieldBlocked`，把 Element Text 替换为 `[REDACTED]`，并确保 Serialized Capture Data 不含测试值。检查后已删除 Test Application 与 Temporary Artifact，Repository Scan 对测试值的匹配数为 0。
