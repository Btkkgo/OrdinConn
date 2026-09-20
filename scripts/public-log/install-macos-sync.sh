#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
project_root="${ORDINCONN_PROJECT_ROOT:-$(git -C "$script_dir" rev-parse --show-toplevel)}"
project_root="$(cd "$project_root" && pwd -P)"
agent_dir="${ORDINCONN_LAUNCH_AGENT_DIR:-$HOME/Library/LaunchAgents}"
log_dir="${ORDINCONN_PUBLIC_LOG_DIR:-$HOME/Library/Logs/OrdinConn}"
label="com.ordinconn.public-devlog-sync"
template="$script_dir/$label.plist.template"
plist="$agent_dir/$label.plist"
sync_script="$project_root/scripts/public-log/sync-public-devlog.sh"
log_path="$log_dir/public-devlog-sync.log"

[ "$(uname -s)" = "Darwin" ] || { echo "MACOS_REQUIRED" >&2; exit 2; }
[ -f "$template" ] || { echo "LAUNCH_AGENT_TEMPLATE_REQUIRED" >&2; exit 2; }
[ -x "$sync_script" ] || { echo "PUBLIC_SYNC_SCRIPT_REQUIRED" >&2; exit 2; }
command -v plutil >/dev/null 2>&1 || { echo "PLUTIL_REQUIRED" >&2; exit 2; }

mkdir -p "$agent_dir" "$log_dir"
temp_plist="$(mktemp "$agent_dir/.$label.XXXXXX")"

python3 - "$template" "$temp_plist" "$sync_script" "$project_root" "$log_path" <<'PY'
from html import escape
from pathlib import Path
import sys

template, output, sync_script, project_root, log_path = sys.argv[1:]
text = Path(template).read_text(encoding="utf-8")
text = text.replace("__SYNC_SCRIPT__", escape(sync_script, quote=True))
text = text.replace("__PROJECT_ROOT__", escape(project_root, quote=True))
text = text.replace("__LOG_PATH__", escape(log_path, quote=True))
Path(output).write_text(text, encoding="utf-8")
PY

if ! plutil -lint "$temp_plist" >/dev/null; then
  python3 - "$temp_plist" <<'PY'
from pathlib import Path
import sys
Path(sys.argv[1]).unlink(missing_ok=True)
PY
  echo "LAUNCH_AGENT_PLIST_INVALID" >&2
  exit 3
fi

mv "$temp_plist" "$plist"

if [ "${ORDINCONN_SKIP_LAUNCHCTL:-0}" != "1" ]; then
  domain="gui/$UID"
  launchctl bootout "$domain" "$plist" >/dev/null 2>&1 || true
  launchctl bootstrap "$domain" "$plist"
  launchctl enable "$domain/$label"
fi

echo "LAUNCH_AGENT_INSTALLED $plist"
