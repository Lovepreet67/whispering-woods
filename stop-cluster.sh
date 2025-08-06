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

echo "Stoping and removing filebeat container"
docker rm -f filebeat >/dev/null 2>&1 || true

echo "Cluster stopped and containers removed."
# for checking available stoage
# sleep 5 
#
# DMG_DIR="$(pwd)/disks"
# MOUNT_BASE="/Volumes"
#
#
# echo "Detaching mounted virtual disks..."
#
# for dmg_path in "$DMG_DIR"/datanode*.dmg; do
#   # Get datanode index from file name
#   filename=$(basename "$dmg_path")
#   index=$(echo "$filename" | sed -E 's/datanode([0-9]+)\.dmg/\1/')
#   mount_point="$MOUNT_BASE/datanode$index"
#
#   if [ -d "$mount_point" ]; then
#     echo "Unmounting $mount_point..."
#     hdiutil detach "$mount_point" 
#   fi
#
#   echo "Deleting $dmg_path..."
#   rm -f "$dmg_path"
# done
