#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "$0")/../../.." && pwd -P)"
installer="$source_root/scripts/github/install-sync-launchagent.sh"
uninstaller="$source_root/scripts/github/uninstall-sync-launchagent.sh"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

project="$test_root/Project & Space"
agent_dir="$test_root/Launch Agents"
log_dir="$test_root/Logs & Output"
python_bin="$test_root/Python & Bin/python3"
mkdir -p "$project/scripts/github" "$agent_dir" "$log_dir" "$(dirname "$python_bin")"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$project/scripts/github/sync-public-log.sh"
chmod +x "$project/scripts/github/sync-public-log.sh"
printf '%s\n' '#!/usr/bin/env bash' 'exec /usr/bin/python3 "$@"' > "$python_bin"
chmod +x "$python_bin"

ORDINCONN_PROJECT_ROOT="$project" \
ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_PUBLIC_LOG_DIR="$log_dir" \
ORDINCONN_PUBLIC_PYTHON_BIN="$python_bin" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$installer" >/dev/null

plist="$agent_dir/com.ordinconn.github-sync.plist"
[ -f "$plist" ] || fail "installer did not create the plist"
plutil -lint "$plist" >/dev/null || fail "rendered plist is invalid"
grep -q '<string>com.ordinconn.github-sync</string>' "$plist" || fail "label is incorrect"
grep -q '<key>RunAtLoad</key>' "$plist" || fail "RunAtLoad is missing"
grep -q '<integer>7200</integer>' "$plist" || fail "two-hour interval is missing"
grep -q 'Project &amp; Space/scripts/github/sync-public-log.sh' "$plist" || fail "script path was not XML escaped"
grep -q 'Logs &amp; Output/github-sync.log' "$plist" || fail "log path was not XML escaped"
grep -q '<key>ORDINCONN_PUBLIC_PYTHON</key>' "$plist" || fail "Python environment key is missing"
grep -q 'Python &amp; Bin/python3' "$plist" || fail "Python path was not XML escaped"

ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$uninstaller" >/dev/null

[ ! -e "$plist" ] || fail "uninstaller did not remove the plist"

echo "PASS: GitHub sync LaunchAgent renders, validates, and uninstalls safely"
