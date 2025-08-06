#!/bin/bash

set -euo pipefail

# Stop Rust processes by their binary names
echo "Stopping Rust services..."

PIDS=$(pgrep -f "target/debug/namenode|target/debug/datanode")
if [ -n "$PIDS" ]; then
  echo "Killing PIDs: $PIDS"
  kill $PIDS
else
  echo "No Rust background services running."
fi

# Stop docker-compose services
echo "Stoping and removing filebeat container"
docker rm -f filebeat >/dev/null 2>&1 || true
