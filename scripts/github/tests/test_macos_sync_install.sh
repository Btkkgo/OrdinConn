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
automation_dir="$test_root/Application Support/OrdinConn Automation"
python_bin="$test_root/Python & Bin/python3"
mkdir -p "$project/scripts/github" "$agent_dir" "$log_dir" "$automation_dir" "$(dirname "$python_bin")"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$project/scripts/github/sync-public-log.sh"
chmod +x "$project/scripts/github/sync-public-log.sh"
printf '%s\n' '#!/usr/bin/env bash' 'exec /usr/bin/python3 "$@"' > "$python_bin"
chmod +x "$python_bin"

ORDINCONN_PROJECT_ROOT="$project" \
ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_PUBLIC_LOG_DIR="$log_dir" \
ORDINCONN_AUTOMATION_DIR="$automation_dir" \
ORDINCONN_PUBLIC_PYTHON_BIN="$python_bin" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$installer" >/dev/null

plist="$agent_dir/com.ordinconn.github-sync.plist"
[ -f "$plist" ] || fail "installer did not create the plist"
plutil -lint "$plist" >/dev/null || fail "rendered plist is invalid"
grep -q '<string>com.ordinconn.github-sync</string>' "$plist" || fail "label is incorrect"
grep -q '<key>RunAtLoad</key>' "$plist" || fail "RunAtLoad is missing"
grep -q '<integer>7200</integer>' "$plist" || fail "two-hour interval is missing"
grep -q 'Application Support/OrdinConn Automation/GitHub Sync.app/Contents/MacOS/github-sync-launcher' "$plist" || fail "dedicated launcher path is missing"
grep -q 'Application Support/OrdinConn Automation/github-sync-runner.sh' "$plist" || fail "Application Support runner path is missing"
grep -q '<key>WorkingDirectory</key>' "$plist" || fail "working directory is missing"
grep -q '<string>.*Application Support/OrdinConn Automation</string>' "$plist" || fail "working directory is not outside the protected checkout"
if grep -q '<string>/bin/bash</string>' "$plist"; then
  fail "LaunchAgent must not use the shared bash executable as its TCC identity"
fi
grep -q 'Logs &amp; Output/github-sync.log' "$plist" || fail "log path was not XML escaped"

runner="$automation_dir/github-sync-runner.sh"
launcher="$automation_dir/GitHub Sync.app/Contents/MacOS/github-sync-launcher"
[ -x "$runner" ] || fail "installer did not create an executable Application Support runner"
[ -x "$launcher" ] || fail "installer did not create the dedicated launcher executable"
grep -q 'Project & Space/scripts/github/sync-public-log.sh' "$runner" || fail "runner does not call the repository sync entry point"
grep -q 'ORDINCONN_PUBLIC_PYTHON=' "$runner" || fail "runner does not set the explicit Python path"
grep -q 'PATH=' "$runner" || fail "runner does not set an explicit PATH"
grep -q 'max_log_bytes=' "$runner" || fail "runner does not rotate its log"
grep -q 'scan=' "$runner" || fail "runner does not write structured scan status"
grep -q 'changes=' "$runner" || fail "runner does not write structured change status"
grep -q 'commit=' "$runner" || fail "runner does not write structured commit status"
grep -q 'push=' "$runner" || fail "runner does not write structured push status"

ORDINCONN_LAUNCH_AGENT_DIR="$agent_dir" \
ORDINCONN_AUTOMATION_DIR="$automation_dir" \
ORDINCONN_SKIP_LAUNCHCTL=1 \
"$uninstaller" >/dev/null

[ ! -e "$plist" ] || fail "uninstaller did not remove the plist"
[ ! -e "$runner" ] || fail "uninstaller did not remove the local runner"
[ ! -e "$automation_dir/GitHub Sync.app" ] || fail "uninstaller did not remove the dedicated launcher"

echo "PASS: GitHub sync LaunchAgent renders, validates, and uninstalls safely"
