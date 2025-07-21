#!/bin/bash

set -e

# this scrip assumes that you have gfs-namenode and gfs-datanode present in docker and ek (elastic search and kibana are running) 
# Set ENV variable globally
export ENV="container"

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
  -e CONFIG_PATH="./container.yaml" \
  -v "$(pwd)/namenode/config/container.yaml":/app/container.yaml \
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
for ((i = 0; i < DATANODE_COUNT; i++)); do
  grpc_port=$((3000 + i*10))
  tcp_port=$((3001 + i*10))
  echo "Starting $i DataNode..."

  docker run -d \
    --name gfs-datanode-$i \
    -p ${grpc_port}:3000 \
    -p ${tcp_port}:3001 \
    -e ENV=$ENV \
    -e CONFIG_PATH="./container.yaml" \
    -v "$(pwd)/cluster_configs/datanode/datanode${i}.yaml":/app/container.yaml \
    gfs-datanode
done


echo "Cluster started with $DATANODE_COUNT datanodes."
echo "Starting a client"
export NAMENODE_ADDRS="http://host.docker.internal:7000"
ENV=default RUST_LOG=client=trace,utilities=trace  APM_ENDPOINT=$APM_ENDPOINT cargo run -p client
