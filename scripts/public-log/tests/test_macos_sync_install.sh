#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
installer="$source_root/scripts/public-log/install-macos-sync.sh"
uninstaller="$source_root/scripts/public-log/uninstall-macos-sync.sh"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[ -x "$installer" ] || fail "installer is missing or not executable"
[ -x "$uninstaller" ] || fail "uninstaller is missing or not executable"

project="$test_root/Project & Space"
agent_dir="$test_root/Launch Agents"
log_dir="$test_root/Logs & Output"
python_bin="$test_root/Python & Bin/python3"
mkdir -p "$project/scripts/public-log" "$agent_dir" "$log_dir" "$(dirname "$python_bin")"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$project/scripts/public-log/sync-public-devlog.sh"
chmod +x "$project/scripts/public-log/sync-public-devlog.sh"
printf '%s\n' '#!/usr/bin/env bash' 'exec /usr/bin/python3 "$@"' > "$python_bin"
chmod +x "$python_bin"

ORDINCONN_PROJECT_ROOT="$project" \
ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_PUBLIC_LOG_DIR="$log_dir" \
ORDINCONN_PUBLIC_PYTHON_BIN="$python_bin" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$installer" >/dev/null

plist="$agent_dir/com.ordinconn.public-devlog-sync.plist"
[ -f "$plist" ] || fail "installer did not create the plist"
plutil -lint "$plist" >/dev/null || fail "rendered plist is invalid"
grep -q '<key>RunAtLoad</key>' "$plist" || fail "RunAtLoad is missing"
grep -q '<key>StartInterval</key>' "$plist" || fail "StartInterval is missing"
grep -q '<integer>7200</integer>' "$plist" || fail "two-hour interval is missing"
grep -q 'Project &amp; Space/scripts/public-log/sync-public-devlog.sh' "$plist" || fail "script path was not XML escaped"
grep -q 'Logs &amp; Output/public-devlog-sync.log' "$plist" || fail "log path was not XML escaped"
grep -q '<key>ORDINCONN_PUBLIC_PYTHON</key>' "$plist" || fail "Python environment key is missing"
grep -q 'Python &amp; Bin/python3' "$plist" || fail "Python path was not XML escaped"

ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$uninstaller" >/dev/null

[ ! -e "$plist" ] || fail "uninstaller did not remove the plist"

echo "PASS: LaunchAgent renders, validates, and uninstalls safely"
