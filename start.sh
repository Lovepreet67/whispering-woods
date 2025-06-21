#!/bin/bash

set -euo pipefail

# Enable logging for all crates at debug level
export RUST_LOG=client=trace,datanode=trace,namenode=trace,utilities=trace

export NAMENODE_ADDRS="http://127.0.0.1:7000" #will work for all
export BASE_URL="127.0.0.1" # added base address for all
export ENV="local"
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
GRPC_PORT="7000" cargo run -p namenode >/dev/null 2>&1 < /dev/null &

sleep 5

# Run DataNode 1 (client port 8000, internal 3000)
echo "Starting DataNode 1..."
GRPC_PORT="8000" TCP_PORT="3000" NODE_ID="datanode_1" cargo run -p datanode >/dev/null 2>&1 < /dev/null&

# Run DataNode 2 (client port 9000, internal 4000)
echo "Starting DataNode 2..."
GRPC_PORT="9000" TCP_PORT="4000" NODE_ID="datanode_2"  cargo run -p datanode >/dev/null 2>&1 < /dev/null&

# Optional delay to let nodes start
sleep 2

# Run Client (in foreground)
echo "Starting Client..."
cargo run -p client 
