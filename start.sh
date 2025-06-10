#!/bin/bash

set -euo pipefail

# Enable logging for all crates at debug level
export RUST_LOG=client=trace,datanode=trace,namenode=trace,utilities=trace

# Define project root and docker-compose file path
DOCKER_COMPOSE_FILE="./logger/docker-compose.yaml"

# Check if docker-compose containers are running
if ! docker compose -f "$DOCKER_COMPOSE_FILE" ps | grep -q "filebeat"; then
    echo "Starting docker-compose stack..."
    docker compose -f "$DOCKER_COMPOSE_FILE" up -d
else
    echo "Docker services already running."
fi

# Wait until Elasticsearch is available
echo "Waiting for Elasticsearch..."
until curl -s http://localhost:9200 >/dev/null; do
    sleep 1
done
echo "Elasticsearch is ready."

# Run NameNode (port 7000)
echo "Starting NameNode..."
cargo run -p namenode -- 7000 >/dev/null 2>&1 < /dev/null &

sleep 5

# Run DataNode 1 (client port 8000, internal 3000)
echo "Starting DataNode 1..."
cargo run -p datanode -- 8000 3000 >/dev/null 2>&1 < /dev/null&

# Run DataNode 2 (client port 9000, internal 4000)
echo "Starting DataNode 2..."
cargo run -p datanode -- 9000 4000 >/dev/null 2>&1 < /dev/null&

# Optional delay to let nodes start
sleep 2

# Run Client (in foreground)
echo "Starting Client..."
cargo run -p client -- http://127.0.0.1:7000
