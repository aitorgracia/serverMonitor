#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "=== Pull latest ==="
git pull

echo "=== Build release ==="
cargo build --release -p monitor_agent

echo "=== Restart service ==="
sudo systemctl restart monitor_agent

echo "=== Status ==="
sudo systemctl status monitor_agent --no-pager

echo "=== Done ==="
