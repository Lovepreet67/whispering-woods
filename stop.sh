#!/bin/bash

set -euo pipefail

DOCKER_COMPOSE_FILE="./logger/docker-compose.yaml"

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
echo "Stopping docker-compose services..."
docker compose -f "$DOCKER_COMPOSE_FILE" down
