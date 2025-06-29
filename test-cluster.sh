#!/bin/bash

# Chaos testing script for GFS cluster
# Requirements: Pumba installed and Docker running with cluster already up
# This script assumes your datanodes are named gfs-datanode-1, gfs-datanode-2, ...

set -e

DATANODE_COUNT=${1:-3}  # default to 3 if not specified
CHAOS_DURATION=${2:-180}  # total test time in seconds
DELAY_BETWEEN_ATTACKS=5  # seconds between chaos rounds

echo "Running chaos test on $DATANODE_COUNT datanodes for $CHAOS_DURATION seconds"

END_TIME=$((SECONDS + CHAOS_DURATION))

while [ $SECONDS -lt $END_TIME ]; do
  for ((i = 1; i <= DATANODE_COUNT; i++)); do
    CONTAINER="gfs-datanode-$i"

    CHAOS_TYPE=$((RANDOM % 3))

    case $CHAOS_TYPE in
      0)
        echo "⛔ Killing $CONTAINER"
        docker kill $CONTAINER
        sleep 1
        echo "🔁 Restarting $CONTAINER"
        docker start $CONTAINER
        ;;
      1)
        echo "⏸️ Pausing $CONTAINER for 10s"
        docker pause $CONTAINER
        sleep 1 
        docker unpause $CONTAINER
        ;;
      2)
        echo "🐢 Adding 500ms network delay to $CONTAINER"
        docker exec $CONTAINER tc qdisc add dev eth0 root netem delay 500ms || true
        sleep 3
        docker exec $CONTAINER tc qdisc del dev eth0 root netem || true
        ;;
    esac
    sleep 1
  done

  echo "🌀 Waiting $DELAY_BETWEEN_ATTACKS seconds before next chaos round..."
  sleep $DELAY_BETWEEN_ATTACKS
done

echo "✅ Chaos test completed."
