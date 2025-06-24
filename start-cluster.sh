#!/bin/bash

set -e

# this scrip assumes that you have gfs-namenode and gfs-datanode present in docker and ek (elastic search and kibana are running) 
# Set ENV variable globally
export ENV="Cluster"

# Here we are mapping host.docker.internal to resolve to local host on local machine 
# so that we can use same address inside container and host machine
grep -Fxq "127.0.0.1 host.docker.internal" /etc/hosts || echo "127.0.0.1 host.docker.internal" | sudo tee -a /etc/hosts

# -----------------------
# Start the namenode
# -----------------------
echo "starting namenode"
docker run -d \
  -p 7000:7000 \
  -e ENV=$ENV \
  -e INTERNAL_GRPC_PORT=7000 \
  -e NODE_ID=namenode_0 \
  -e RUST_LOG=namenode=trace \
  --name gfs-namenode \
  gfs-namenode
echo "Namenode running"

# Wait for namenode to boot up
sleep 3 

# -----------------------
# Start a datanode
# -----------------------

DATANODE_COUNT=${1:-3}  # default to 3 if no argument is passed
echo "Starting $DATANODE_COUNT DataNodes..."
for ((i = 1; i <= DATANODE_COUNT; i++)); do
  grpc_port=$((3000 + i * 10))
  tcp_port=$((3001 + i * 10))
  echo "Starting $i DataNode..."

  docker run -d \
    --name gfs-datanode-$i \
    -p ${grpc_port}:3000 \
    -p ${tcp_port}:3001 \
    -e NODE_ID=datanode_$i \
    -e INTERNAL_GRPC_PORT=3000 \
    -e EXTERNAL_GRPC_ADDRS=http://host.docker.internal:${grpc_port} \
    -e INTERNAL_TCP_PORT=3001 \
    -e EXTERNAL_TCP_ADDRS=host.docker.internal:${tcp_port} \
    -e NAMENODE_ADDRS=http://host.docker.internal:7000 \
    -e RUST_LOG=datanode=trace \
    gfs-datanode
done


echo "Cluster started with $DATANODE_COUNT datanodes."
echo "Starting a client"
export NAMENODE_ADDRS="http://host.docker.internal:7000"
RUST_LOG=client=trace cargo run -p client
