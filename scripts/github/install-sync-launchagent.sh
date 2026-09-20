#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
project_root="${ORDINCONN_PROJECT_ROOT:-$(git -C "$script_dir" rev-parse --show-toplevel)}"
project_root="$(cd "$project_root" && pwd -P)"
agent_dir="${ORDINCONN_LAUNCH_AGENT_DIR:-$HOME/Library/LaunchAgents}"
log_dir="${ORDINCONN_PUBLIC_LOG_DIR:-$HOME/Library/Logs/OrdinConn}"
automation_dir="${ORDINCONN_AUTOMATION_DIR:-$HOME/Library/Application Support/OrdinConn/automation}"
python_bin="${ORDINCONN_PUBLIC_PYTHON_BIN:-$(command -v python3 || true)}"
git_bin="${ORDINCONN_PUBLIC_GIT_BIN:-$(command -v git || true)}"
developer_dir="${ORDINCONN_DEVELOPER_DIR:-$(xcode-select -p 2>/dev/null || true)}"
clang_bin="${ORDINCONN_CLANG_BIN:-$(xcrun --find clang 2>/dev/null || command -v clang || true)}"
codesign_bin="${ORDINCONN_CODESIGN_BIN:-$(command -v codesign || true)}"
label="com.ordinconn.github-sync"
template="$script_dir/$label.plist.template"
runner_template="$script_dir/github-sync-runner.sh.template"
launcher_source="$script_dir/macos/github-sync-launcher.c"
launcher_info="$script_dir/macos/GitHubSyncLauncher-Info.plist"
plist="$agent_dir/$label.plist"
sync_script="$project_root/scripts/github/sync-public-log.sh"
log_path="$log_dir/github-sync.log"
runner_path="$automation_dir/github-sync-runner.sh"
launcher_app="$automation_dir/GitHub Sync.app"
launcher_bin="$launcher_app/Contents/MacOS/github-sync-launcher"

[ "$(uname -s)" = "Darwin" ] || { echo "MACOS_REQUIRED" >&2; exit 2; }
[ -f "$template" ] || { echo "LAUNCH_AGENT_TEMPLATE_REQUIRED" >&2; exit 2; }
[ -f "$runner_template" ] || { echo "GITHUB_SYNC_RUNNER_TEMPLATE_REQUIRED" >&2; exit 2; }
[ -f "$launcher_source" ] || { echo "GITHUB_SYNC_LAUNCHER_SOURCE_REQUIRED" >&2; exit 2; }
[ -f "$launcher_info" ] || { echo "GITHUB_SYNC_LAUNCHER_INFO_REQUIRED" >&2; exit 2; }
[ -x "$sync_script" ] || { echo "GITHUB_SYNC_SCRIPT_REQUIRED" >&2; exit 2; }
[ -n "$python_bin" ] && [ -x "$python_bin" ] || { echo "PYTHON3_REQUIRED" >&2; exit 2; }
[ -n "$git_bin" ] && [ -x "$git_bin" ] || { echo "GIT_REQUIRED" >&2; exit 2; }
[ -n "$clang_bin" ] && [ -x "$clang_bin" ] || { echo "CLANG_REQUIRED" >&2; exit 2; }
[ -n "$codesign_bin" ] && [ -x "$codesign_bin" ] || { echo "CODESIGN_REQUIRED" >&2; exit 2; }
command -v plutil >/dev/null 2>&1 || { echo "PLUTIL_REQUIRED" >&2; exit 2; }

mkdir -p "$agent_dir" "$log_dir" "$automation_dir"
temp_plist="$(mktemp "$agent_dir/.$label.XXXXXX")"
temp_runner="$(mktemp "$automation_dir/.github-sync-runner.XXXXXX")"
temp_app="$(mktemp -d "$automation_dir/.github-sync-launcher.XXXXXX")"
trap 'rm -f -- "$temp_plist" "$temp_runner"; rm -rf -- "$temp_app"' EXIT

path_value="$(dirname "$git_bin"):$(dirname "$python_bin"):/usr/bin:/bin:/usr/sbin:/sbin"

"$python_bin" - "$template" "$temp_plist" "$runner_template" "$temp_runner" "$launcher_bin" "$runner_path" "$automation_dir" "$project_root" "$sync_script" "$log_path" "$git_bin" "$python_bin" "$path_value" <<'PY'
from html import escape
from pathlib import Path
from shlex import quote
import sys

(
    plist_template,
    plist_output,
    runner_template,
    runner_output,
    launcher_bin,
    runner_path,
    automation_dir,
    project_root,
    sync_script,
    log_path,
    git_bin,
    python_bin,
    path_value,
) = sys.argv[1:]

plist_text = Path(plist_template).read_text(encoding="utf-8")
plist_text = plist_text.replace("__LAUNCHER_BIN__", escape(launcher_bin, quote=True))
plist_text = plist_text.replace("__RUNNER_PATH__", escape(runner_path, quote=True))
plist_text = plist_text.replace("__AUTOMATION_DIR__", escape(automation_dir, quote=True))
plist_text = plist_text.replace("__LOG_PATH__", escape(log_path, quote=True))
Path(plist_output).write_text(plist_text, encoding="utf-8")

runner_text = Path(runner_template).read_text(encoding="utf-8")
replacements = {
    "__PROJECT_ROOT_SHELL__": quote(project_root),
    "__SYNC_SCRIPT_SHELL__": quote(sync_script),
    "__LOG_PATH_SHELL__": quote(log_path),
    "__GIT_BIN_SHELL__": quote(git_bin),
    "__PYTHON_BIN_SHELL__": quote(python_bin),
    "__PATH_VALUE_SHELL__": quote(path_value),
}
for placeholder, value in replacements.items():
    runner_text = runner_text.replace(placeholder, value)
Path(runner_output).write_text(runner_text, encoding="utf-8")
PY

if ! plutil -lint "$temp_plist" >/dev/null; then
  "$python_bin" - "$temp_plist" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).unlink(missing_ok=True)
PY
  echo "LAUNCH_AGENT_PLIST_INVALID" >&2
  exit 3
fi

mkdir -p "$temp_app/Contents/MacOS"
cp "$launcher_info" "$temp_app/Contents/Info.plist"
clang_args=(-O2 -Wall -Wextra)
if [ -d "$developer_dir/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk" ]; then
  clang_args+=(-isysroot "$developer_dir/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk")
fi
"$clang_bin" "${clang_args[@]}" -o "$temp_app/Contents/MacOS/github-sync-launcher" "$launcher_source"
"$codesign_bin" --force --sign - --identifier com.ordinconn.github-sync-runner "$temp_app" >/dev/null

chmod 700 "$temp_runner"
rm -rf -- "$launcher_app"
mv "$temp_app" "$launcher_app"
temp_app=""
mv "$temp_runner" "$runner_path"
temp_runner=""
mv "$temp_plist" "$plist"
temp_plist=""

if [ "${ORDINCONN_SKIP_LAUNCHCTL:-0}" != "1" ]; then
  domain="gui/$UID"
  lsregister="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
  [ ! -x "$lsregister" ] || "$lsregister" -f "$launcher_app" >/dev/null 2>&1 || true
  launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  launchctl bootstrap "$domain" "$plist"
  launchctl enable "$domain/$label"
fi

echo "LAUNCH_AGENT_INSTALLED $plist"
echo "GITHUB_SYNC_RUNNER_INSTALLED $runner_path"
echo "GITHUB_SYNC_LAUNCHER_INSTALLED $launcher_app"
