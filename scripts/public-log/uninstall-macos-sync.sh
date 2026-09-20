#!/usr/bin/env bash
set -euo pipefail

agent_dir="${ORDINCONN_LAUNCH_AGENT_DIR:-$HOME/Library/LaunchAgents}"
label="com.ordinconn.public-devlog-sync"
plist="$agent_dir/$label.plist"

[ "$(uname -s)" = "Darwin" ] || { echo "MACOS_REQUIRED" >&2; exit 2; }

if [ "${ORDINCONN_SKIP_LAUNCHCTL:-0}" != "1" ] && [ -e "$plist" ]; then
  launchctl bootout "gui/$UID" "$plist" >/dev/null 2>&1 || true
fi

python3 - "$plist" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).unlink(missing_ok=True)
PY

echo "LAUNCH_AGENT_UNINSTALLED $plist"
