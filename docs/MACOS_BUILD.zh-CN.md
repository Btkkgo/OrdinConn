# macOS 验收构建 — 仅保留最新应用

[English](MACOS_BUILD.md) | [简体中文](MACOS_BUILD.zh-CN.md)

## 永久规则

每次生成新的 OrdinConn macOS 可运行验收版本后，只保留最新有效应用版本。旧 app bundle 在确认来源和过期状态后清理，避免 Spotlight、Launch Services 和用户产生多个 OrdinConn 启动入口。清理仅针对可重新生成的应用构建产物，永远不得删除源码、Git 仓库、用户数据、数据库、Keychain、凭据、Settings、配置、Evidence、DevLog、测试记录、文档或截图/JPEG 验收证据。来源不明必须 fail closed，并交由 owner 确认。

维护此规则时，只能编辑明确授权的开发/构建文档和脚本。应用清理不得写业务数据或修改 Provider 配置。正常启动应用是需要 owner 授权的独立验收动作。

## 支持的工作流

退出 OrdinConn，然后在仓库根目录执行：

```sh
npm run desktop:build
```

workspace 的 `tauri:build` 使用同一个包装脚本。macOS 要求 Python 3.9+ 和已安装的 Apple 命令行工具。默认保留 `target/release/bundle/macos/OrdinConn.app`。`npm run tauri:build --workspace @ordinconn/desktop -- --debug` 改为保留新的 debug 包，并只清理已证明过期的 release 包。其他系统保持调用现有 Tauri CLI。

包装脚本依次执行：

1. 只读盘点 `/Applications`、`~/Applications`、项目产物目录、Launch Services 和 Spotlight；构建前检查每一个现存候选。运行中的应用、符号链接、Git 跟踪文件、未知位置或未知来源都会阻止操作。
2. 从当前 HEAD **及当前工作区源码指纹**构建原生应用包，明确记录 dirty 状态。若构建期间源码变化则拒绝清理。自定义 target 目录、交叉编译、自定义打包参数或非预期打包配置需要单独审查工作流。
3. 验证应用身份、原生可执行文件、本地 ad-hoc 签名和包内容。为本地应用包完成 ad-hoc 签名，要求 `codesign --verify --deep --strict` 通过。这是本地验收签名，不是分发签名或公证。
4. 在 `target/ordinconn-build-receipts/<bundle-hash>.<build-timestamp>.json` 保存脱敏后的 HEAD、源码指纹、dirty 标记、UTC 构建时间、可执行文件 hash 和完整包 hash。每次构建单独保留一份记录，即使可执行文件字节相同也不覆盖旧记录；不存储凭据、机器主目录绝对路径或业务数据。
5. 重新盘点，证明每个待删除包在构建前已存在、内容未变且早于已验证最新包。允许删除的位置仅限本 checkout 根 target 或 desktop target 下的准确 debug/release 包路径，以及有既有构建记录证明文件指纹的 `/Applications/OrdinConn.app` 和 `~/Applications/OrdinConn.app`。没有构建记录的旧项目标准产物还必须与签名的 Cargo 可执行文件字节完全一致，并符合最小已知资源布局。文件名、版本号或时间戳单独不能证明来源。
6. 注销已证明过期的包，再次检查完整文件指纹，仅删除该包。禁止全局重置 Launch Services、大范围清空缓存或目录清理。清除应用文件已不存在的 OrdinConn 陈旧登记。
7. 登记并导入保留包；验证现存应用、有效 Launch Services 路径和 Spotlight 结果各恰好一个。登记或索引失败保持 BLOCKED，不得只根据登记命令声称成功。

保留的项目应用包就是实际使用入口，工作流不创建第二个安装副本。安装副本来源不明时，保留并停止等待审查。构建不会自动启动应用、操作 Keychain，或初始化/迁移 SQLite。获得授权并启动后，另行验证 Settings → Models、现有 Provider 元数据和业务记录保留。

## 冻结 M3 验收 Binary

最终验收构建通过应用包和登记验证后，在 `target/final-m3-gate/FINAL_M3_BINARY_FROZEN.json` 记录冻结身份。记录 schema version `1`、state `FROZEN`、相对应用路径、可执行文件 SHA256、完整应用包指纹、source HEAD、源码指纹、dirty-worktree 指纹、UTC 构建/冻结时间以及签名身份。这份本地记录不包含凭据或业务数据，也不替代构建回执。

Planner 5/5 和 Android Final Gate 必须使用同一个应用可执行文件；两个 Gate 均结束前禁止重新构建。macOS 构建开始前，包装脚本检查冻结记录：有效冻结记录以 `FINAL_M3_BINARY_FROZEN` 停止；未知、不完整、损坏或符号链接记录同样阻止重新构建。记录不存在时保留原工作流，只读审计仍可使用。包装脚本不提供冻结 bypass、解冻或删除 marker 选项。Keychain 授权和 Gate 执行仍是独立、需 owner 授权的动作。

## 只读审计与测试

```sh
python3 scripts/desktop/macos_latest.py --audit
python3 -m unittest discover -s scripts/desktop/tests -v
git diff --check
```

策略测试使用可丢弃 fixture，覆盖符号链接、未知资源、源码跟踪、包内容变化、来源缺失、较新时间戳和受保护数据库。它们不删除真实应用，也不验证真实 Launch Services。真实验收仍需 macOS 打包、登记、Spotlight、UI 和数据检查。不能将构建或索引失败记为 PASS，也不能为通过检查删除来源不明应用。
