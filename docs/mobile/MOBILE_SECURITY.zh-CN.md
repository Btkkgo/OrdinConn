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

## M2 动作边界

M2 仍只允许 Emulator、人工触发、Allowlist、绑定快照，每 Session 最多 20 次已派发尝试。生产环境的非逃离导航进一步只支持 Android Settings 与 Settings Intelligence；用户新增任意 App 不会获得动作权限。同一 Activity 内实时 UI Tree 变化会阻断旧坐标输入。敏感/金融屏幕只允许 Back/Home 安全退出；常见中英文支付与凭据文案会脱敏，但语义匹配不能保证覆盖所有语言。脱敏 Pending Receipt 与审计 Intent 在 ADB 输入前持久写入；输入前写入失败则不输入，结果写入失败会留下 Pending Receipt 并停止逻辑会话。真实 M2 密码字段 Type 尝试在 ADB 输入前被阻断；随机测试文本未出现在序列化 Capture、Receipt、Workspace 或 SQLite 中。一次性测试 APK 由验收流程安装而非生产 Runtime 安装，随后卸载。见 [M2 验收记录](M2_ACCEPTANCE.zh-CN.md)。动作后采集失败不会让动作 Verified，也不会自动重试。
