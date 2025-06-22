#!/bin/bash
# this scrip assumes that you have gfs-namenode and gfs-datanode present in docker and ek (elastic search and kibana are running) 
# Set ENV variable globally
export ENV="Cluster"

# -----------------------
# Start the namenode
# -----------------------
docker run -d \
  -p 7000:7000 \
  -e ENV=$ENV \
  -e GRPC_PORT=7000 \
  -e NODE_ID=namenode_0 \
  -e BASE_URL="127.0.0.1" \
  -e RUST_LOG=namenode=trace \
  --name gfs-namenode \
  gfs-namenode

# Wait for namenode to boot up
sleep 3 

# -----------------------
# Start a datanode
# -----------------------
docker run -d \
  -p 3000:3000 \
  -p 3001:3001 \
  -e ENV=$ENV \
  -e GRPC_PORT=3000 \
  -e TCP_PORT=3001 \
  -e BASE_URL="127.0.0.1" \
  -e NODE_ID=datanode_1 \
  -e RUST_LOG=datanode=trace \
  -e NAMENODE_ADDRS=http://host.docker.internal:7000 \
  --name gfs-datanode-1 \
  gfs-datanode
