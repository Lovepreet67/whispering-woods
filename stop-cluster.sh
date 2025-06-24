#!/bin/bash

echo "Stopping and removing NameNode..."
docker rm -f gfs-namenode >/dev/null 2>&1 || true

echo "Stopping and removing DataNodes..."

# Find all running/stopped containers that match "gfs-datanode-*"
datanodes=$(docker ps -a --format '{{.Names}}' | grep '^gfs-datanode-' || true)

if [ -z "$datanodes" ]; then
  echo "No datanode containers found."
else
  for name in $datanodes; do
    docker rm -f "$name" >/dev/null 2>&1 || true
    echo "Removed $name"
  done
fi

echo "Cluster stopped and containers removed."

