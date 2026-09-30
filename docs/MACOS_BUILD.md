# macOS Acceptance Builds — One Latest App

[English](MACOS_BUILD.md) | [简体中文](MACOS_BUILD.zh-CN.md)

## Permanent rule

After each new runnable OrdinConn macOS acceptance build, retain only the latest verified application. Remove an older app bundle only after proving its origin and staleness, so Spotlight, Launch Services, and the user have one OrdinConn launch target. Cleanup applies only to reproducible application bundles. Never delete source, Git repositories, user data, databases, Keychain, credentials, settings, configuration, Evidence, DevLogs, test records, documentation, or screenshot/JPEG acceptance evidence. Unknown provenance fails closed and requires owner review.

Only the explicitly requested development/build documentation and scripts may be edited when maintaining this rule. App cleanup must not write business data or configure providers. Normal application startup remains a separate owner-authorized acceptance action.

## Supported workflow

Quit OrdinConn, then run from the repository root:

```sh
npm run desktop:build
```

The workspace `tauri:build` command uses the same wrapper. On macOS it requires Python 3.9+ and Apple's installed command-line tools. The default retained target is `target/release/bundle/macos/OrdinConn.app`. `npm run tauri:build --workspace @ordinconn/desktop -- --debug` retains the new debug bundle instead and removes only a proven stale release bundle. Non-macOS builds pass through to the existing Tauri CLI.

The wrapper performs:

1. Read-only inventory of `/Applications`, `~/Applications`, project output locations, Launch Services, and Spotlight; check every existing candidate before building. Running applications, symlinks, tracked files, unknown locations, or unknown provenance block the operation.
2. Build the native bundle from the current HEAD **and current working-tree source fingerprint**. Dirty changes remain explicit. Reject a source change during the build. Custom target directories, cross-compilation, custom bundler arguments, and unexpected bundle configuration require a separate reviewed workflow.
3. Verify identity, native executable, local ad-hoc signature, and bundle contents. Seal the local bundle with an ad-hoc signature and require `codesign --verify --deep --strict` to pass. This is local acceptance signing, not distribution signing or notarization.
4. Save sanitized HEAD, source fingerprint, dirty flag, UTC build time, executable hash, and complete bundle hash in `target/ordinconn-build-receipts/<bundle-hash>.<build-timestamp>.json`. Each build gets a separate retained record, even when executable bytes are identical; no credential, absolute home path, or business data is stored.
5. Re-inventory and prove that each removable bundle existed unchanged before the build and is older than the verified latest bundle. Allowed deletion locations are the exact debug/release bundle paths under this checkout's root target or desktop target, plus `/Applications/OrdinConn.app` and `~/Applications/OrdinConn.app` when an existing build receipt proves their bytes. Legacy canonical project outputs without receipts additionally require an exact match to their signed Cargo executable and a minimal known resource layout. A filename, version number, or timestamp alone is never proof.
6. Unregister each verified stale bundle, recheck its complete file fingerprint, and remove only that bundle. Never use global Launch Services resets, broad cache purges, or directory-wide cleanup. Remove stale OrdinConn registrations whose app no longer exists.
7. Register and import the retained app. Verify exactly one existing application, one valid Launch Services path, and one Spotlight result. Registration or indexing failures remain BLOCKED; do not infer success from a registration command alone.

The retained project bundle is the actual user-facing launch target; the workflow does not create a second installed copy. If an installed copy has unknown provenance, preserve it and stop for review. The build does not automatically launch the app, touch Keychain, or initialize/migrate SQLite. After an authorized launch, separately verify Settings → Models and preservation of existing provider metadata and business records.

## Read-only audit and tests

```sh
python3 scripts/desktop/macos_latest.py --audit
python3 -m unittest discover -s scripts/desktop/tests -v
git diff --check
```

Policy tests use disposable fixtures, including symlinks, unknown resources, source tracking, changed bundles, missing provenance, newer timestamps, and protected databases. They do not delete real applications or validate real Launch Services. Real macOS packaging, registration, Spotlight, and UI/data checks are required for live acceptance. Do not mark a failed build or indexing check as PASS, and do not delete an ambiguous app to make a check pass.
