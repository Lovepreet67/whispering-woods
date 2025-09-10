#!/bin/bash

set -e

# this scrip assumes that you have gfs-namenode and gfs-datanode present in docker and ek (elastic search and kibana are running) 
# Set ENV variable globally
export ENV="container"

# Here we are mapping host.docker.internal to resolve to local host on local machine 
# so that we can use same address inside container and host machine
# For mac os and linux
grep -Fxq "127.0.0.1 host.docker.internal" /etc/hosts || echo "127.0.0.1 host.docker.internal" | sudo tee -a /etc/hosts

# For windows system will wsl (must run as administrator)
# grep -Fxq "127.0.0.1 host.docker.internal" /mnt/c/Windows/System32/drivers/etc/hosts \
#   || echo "127.0.0.1 host.docker.internal" | tee -a /mnt/c/Windows/System32/drivers/etc/hosts

# For gitbash/minGW (must run as administrator)
#grep -Fxq "127.0.0.1 host.docker.internal" /c/Windows/System32/drivers/etc/hosts \
#  || echo "127.0.0.1 host.docker.internal" | tee -a /c/Windows/System32/drivers/etc/hosts


#change this log volume mapping if you are using other location for logs in default config.yaml logs path it ./temp**
echo "starting the filebeat container"
docker run -d \
  --name=filebeat \
  --user=root \
  --volume="$(pwd)/logger/filebeat/filebeat.yml:/usr/share/filebeat/filebeat.yml:ro" \
  --volume="./temp/:/logs/:ro" \
  docker.elastic.co/beats/filebeat:8.17.7 \
  filebeat
echo "filebeat container is running now"
# -----------------------
# Start the namenode
# -----------------------
echo "starting namenode"
docker run -d \
  -p 7000:7000 \
  -e ENV=$ENV \
  -e RUST_LOG=namenode=trace \
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
DATANODE_COUNT=${1:-3}

echo "Starting $DATANODE_COUNT DataNodes..."
for ((i = 0; i < DATANODE_COUNT; i++)); do
  grpc_port=$((3000 + i*10))
  tcp_port=$((3001 + i*10))

  docker run -d \
    --name gfs-datanode-$i \
    -p ${grpc_port}:3000 \
    -p ${tcp_port}:3001 \
    -e ENV=$ENV \
    -e CONFIG_PATH="./container.yaml" \
    -e RUST_LOG=datanode=trace,storage=trace,utilities=trace \
    -v "$(pwd)/cluster_configs/datanode/datanode${i}.yaml":/app/container.yaml \
    --tmpfs /app/store:rw,size=128m \
    gfs-datanode

done


echo "Cluster started with $DATANODE_COUNT datanodes."
echo "Starting a client"
export NAMENODE_ADDRS="http://host.docker.internal:7000"
ENV=default RUST_LOG=client=trace,utilities=trace  APM_ENDPOINT=$APM_ENDPOINT cargo run -p client
