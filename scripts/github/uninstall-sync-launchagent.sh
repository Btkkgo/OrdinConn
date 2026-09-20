#!/usr/bin/env bash
set -euo pipefail

agent_dir="${ORDINCONN_LAUNCH_AGENT_DIR:-$HOME/Library/LaunchAgents}"
automation_dir="${ORDINCONN_AUTOMATION_DIR:-$HOME/Library/Application Support/OrdinConn/automation}"
label="com.ordinconn.github-sync"
plist="$agent_dir/$label.plist"
runner_path="$automation_dir/github-sync-runner.sh"
launcher_app="$automation_dir/GitHub Sync.app"

if [ "${ORDINCONN_SKIP_LAUNCHCTL:-0}" != "1" ] && [ "$(uname -s)" = "Darwin" ]; then
  launchctl bootout "gui/$UID" "$plist" >/dev/null 2>&1 || true
fi

if [ -e "$plist" ]; then
  rm "$plist"
fi
if [ -e "$runner_path" ]; then
  rm "$runner_path"
fi
if [ -d "$launcher_app" ]; then
  rm -rf -- "$launcher_app"
fi

echo "LAUNCH_AGENT_UNINSTALLED $plist"
