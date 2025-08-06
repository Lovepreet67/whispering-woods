# Monitoring System Setup

This document explains how to set up the monitoring tools for our system.
The setup consists of:
- Three main containers: Elasticsearch, Kibana, and APM Server
- One init container: For setting up credentials and initializing services

All required files for this setup are located in the **./logger/es_kibana_apm_setup directory**.

---
### Pre requisites
- Docker installed and running

Setup Steps

- Change your working directory:
  ```
  cd ./logger/es_kibana_apm_setup 
  ```

- Start the monitoring stack:
    ```
    docker compose up -d
    ```

After completing these steps, you will have a running monitoring system.
Shipping Local Logs (Optional)

If you want to ship logs from your host machine to this monitoring stack you need to Run Filebeat separately for host machine logs.

- This is required because Filebeat embedded in the node containers cannot access host log files.

- A separate Filebeat container is already configured and will run automatically when you execute start.sh or start-cluster.sh.

Note: APM data does not require Filebeat. Processes running on the host can send APM stats directly to the APM server.

----
### Usage

Once the stack is running, access Kibana at:
http://localhost:5601/

**Viewing Logs**

- Go to Discover: http://localhost:5601/app/discover

- In Discover, click Create Data View and select filebeat-* as the data source. If this option is not visible system have not produced any log till now.

**Viewing Performance Metrics (APM)**
- Go to the APM app in Kibana: http://localhost:5601/app/apm/services
- Once your application starts sending APM data, it will appear in this dashboard.
---
**Summary**
- docker compose up -d → Runs Elasticsearch, Kibana, and APM server.
- Filebeat (in-node or separate) → Ships logs to Elasticsearch.
- Kibana → Used for visualizing logs and performance metrics.

---

**_NOTE_**: 

- All the passwords and usernames are present in ./logger/es_kibana_apm/.env, you can update the file inorder to change defaults.

- It is required to pass the new username/password to Datanode, Namenode and filebeat containers.

- To pass new username/password you can use ENV variable ELASTIC_USERNAME and ELASTIC_PASSWORD while running the container.
