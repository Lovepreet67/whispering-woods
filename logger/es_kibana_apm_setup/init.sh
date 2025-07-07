
#!/bin/bash

set -e

ELASTIC_USER=${ELASTIC_USERNAME:-elastic}
ELASTIC_PASS=${ELASTIC_PASSWORD:-changeme}
ELASTIC_HOST=${ELASTICSEARCH_HOST:-http://elasticsearch:9200}
ADMIN_USER=${ADMIN_USERNAME:-my_admin_user}
ADMIN_PASS=${ADMIN_PASSWORD:-admin_pass_123}

# Wait for Elasticsearch to be up
echo "Waiting for Elasticsearch to be available..."
until curl -s -u "$ELASTIC_USER:$ELASTIC_PASS" "$ELASTIC_HOST" | grep -q "cluster_name"; do
  echo -n "."
  sleep 5
done

echo ""
echo "Elasticsearch is up. Resetting passwords..."

# Reset passwords
curl -X POST -u "$ELASTIC_USER:$ELASTIC_PASS" "$ELASTIC_HOST/_security/user/kibana_system/_password" -H "Content-Type: application/json" -d "{\"password\":\"$KIBANA_PASSWORD\"}"
curl -X POST -u "$ELASTIC_USER:$ELASTIC_PASS" "$ELASTIC_HOST/_security/user/apm_system/_password" -H "Content-Type: application/json" -d "{\"password\":\"$APM_PASSWORD\"}"

echo "Passwords reset successfully!"
echo "Creating new super admin user: $ADMIN_USER"

curl -X POST -u "$ELASTIC_USER:$ELASTIC_PASS" "$ELASTIC_HOST/_security/user/$ADMIN_USER" \
  -H "Content-Type: application/json" \
  -d "{
    \"password\": \"$ADMIN_PASS\",
    \"roles\": [\"superuser\"],
    \"full_name\": \"Super Admin User\",
    \"email\": \"admin@example.com\"
  }"

echo "User $ADMIN_USER created successfully!"
