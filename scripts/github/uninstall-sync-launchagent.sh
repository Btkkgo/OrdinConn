#!/usr/bin/env bash
set -euo pipefail

agent_dir="${ORDINCONN_LAUNCH_AGENT_DIR:-$HOME/Library/LaunchAgents}"
label="com.ordinconn.github-sync"
plist="$agent_dir/$label.plist"

if [ "${ORDINCONN_SKIP_LAUNCHCTL:-0}" != "1" ] && [ "$(uname -s)" = "Darwin" ]; then
  launchctl bootout "gui/$UID" "$plist" >/dev/null 2>&1 || true
fi

if [ -e "$plist" ]; then
  rm "$plist"
fi

echo "LAUNCH_AGENT_UNINSTALLED $plist"
