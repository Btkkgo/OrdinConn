#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd -P)"
exec "$script_dir/../public-log/sync-public-devlog.sh" "$@"
