#!/bin/bash

set -euo pipefail

# Enable logging for all crates at debug level
export RUST_LOG=client=trace,datanode=trace,namenode=trace,utilities=trace

export NAMENODE_ADDRS="http://127.0.0.1:7000" #will work for all
export ENV="default"
# Define project root and docker-compose file path

# Wait until Elasticsearch is available
echo "Waiting for Elasticsearch..."
until curl -s http://localhost:9200 >/dev/null; do
    sleep 1
done
echo "Elasticsearch is ready."

echo "starting the filebeat container"
docker run -d \
  --name=filebeat \
  --user=root \
  --volume="$(pwd)/logger/filebeat/filebeat.yml:/usr/share/filebeat/filebeat.yml:ro" \
  --volume="./temp/:/logs/:ro" \
  docker.elastic.co/beats/filebeat:8.17.7 \
  filebeat
echo "filebeat container is running now"

cargo build --release

# Run NameNode (port 7000)
echo "Starting NameNode..."
cargo run -p namenode >/dev/null 2>&1 < /dev/null &

sleep 5

# Run DataNode 1 (client port 8000, internal 3000)
echo "Starting DataNode 1..."
cargo run -p datanode >/dev/null 2>&1 < /dev/null&

# Optional delay to let nodes start
sleep 2

# Run Client (in foreground)
echo "Starting Client..."
cargo run -p client 
