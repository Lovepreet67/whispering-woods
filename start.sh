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

sleep 10

# We will login to the namenode and get jwt to generate certifcates for nodes
NAMENODE_URL="http://localhost:8080"
NAMENODE_USERNAME="username"
NAMENODE_PASSWORD="password"

echo "Logging in..."
LOGIN_RESPONSE=$(curl -s -X POST "$NAMENODE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\": \"$NAMENODE_USERNAME\", \"password\": \"$NAMENODE_PASSWORD\"}")

echo "Logging response..."
TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.Token')
if [[ "$TOKEN" == "null" || -z "$TOKEN" ]]; then
  echo "Failed to extract token from login response"
  echo "$LOGIN_RESPONSE"
  exit 1
fi

echo "Login Successfull."

# Run DataNode 1 (client port 8000, internal 3000)
NODE_ID=datanode_1
echo "Starting DataNode 1..."
echo "Requesting certificate for node '$NODE_ID'..."
  CERT_RESPONSE=$(curl -s -X POST "$NAMENODE_URL/cert/issue" \
    -H "jwt_token: $TOKEN" \
    -H "auth_type: JwtTokenAuth" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\": \"$NODE_ID\", \"node_type\":\"Datanode\"}")

CERT=$(echo "$CERT_RESPONSE" | jq -r '.cert')
SECKEY=$(echo "$CERT_RESPONSE" | jq -r '.key')
if [[ "$CERT" == "null" || -z "$CERT" ]]; then
  echo "Failed to extract cert from response"
  echo "$CERT_RESPONSE"
  continue
fi
echo "Certificate received "

# ---- APPEND RAW CERT TO YAML CONFIG ----
echo "Appending certificate"

cat <<EOF >> "$(pwd)/datanode/config/default.yaml"
namenode_cert: $(echo "$CERT" | sed 's/^/  /')
secret_key: $(echo "$SECKEY" | sed 's/^/  /')
EOF

echo "Certificate appended successfully."

cargo run -p datanode >/dev/null 2>&1 < /dev/null&

# Optional delay to let nodes start
sleep 2


echo "Starting a client"
echo "Getting cert for client"
CERT_RESPONSE=$(curl -s -X POST "$NAMENODE_URL/cert/issue" \
    -H "jwt_token: $TOKEN" \
    -H "auth_type: JwtTokenAuth" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\": \"client_0\", \"node_type\":\"Client\"}")

  CERT=$(echo "$CERT_RESPONSE" | jq -r '.cert')
  SECKEY=$(echo "$CERT_RESPONSE" | jq -r '.key')

  if [[ "$CERT" == "null" || -z "$CERT" ]]; then
   echo "Failed to extract cert from response"
   echo "$CERT_RESPONSE"
   continue
  fi
 echo "Certificate received "

# ---- APPEND RAW CERT TO YAML CONFIG ----
echo " Appending client certificate"

cat <<EOF >> "$(pwd)/client/config/default.yaml"
namenode_cert: $(echo "$CERT" | sed 's/^/  /')
secret_key: $(echo "$SECKEY" | sed 's/^/  /')
EOF

echo "Client Certificate appended successfully."
# Run Client (in foreground)
echo "Starting Client..."
cargo run -p client 
