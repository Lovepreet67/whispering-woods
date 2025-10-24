# Whispering Woods

Whispering Woods is a **distributed file system** inspired by the design principles of the **Google File System (GFS)**.  
It is implemented in **Rust** and provides a scalable, fault‑tolerant way to store, retrieve, and manage large files across multiple machines.  

---
## Features

- **Distributed Storage Architecture** – Namenode & multiple Datanodes.
- **Chunk-Based File Storage** – Files split into fixed-size chunks with unique IDs.
- **Replication** – Multiple replicas stored across Datanodes for reliability.
- **Fault Tolerance** – Heartbeat mechanism to detect failed Datanodes.
- **File Operations**
  - Store file
  - Fetch file
  - Delete file
- **gRPC + TCP Hybrid Communication**
  - gRPC for messages
  - TCP for data plane (chunk streaming)
---

**Roles:**
- **Client** – Provide CLI for user interaction.
- **Namenode** – Maintains metadata, chunk mapping, and coordinates file operations.
- **Datanodes** – Store and serve file chunks; handle replication pipelines.

---

## Technical Details

- **Language:** Rust
- **Networking:**
  - gRPC (Tonic) – Message communication
  - TCP – Chunk transfer
- **Persistence:** Namenode stores metadata in a ledger , in order to recover itself on failure
- **Replication:** Pipeline replication between Datanodes
- **Fault Detection:** Heartbeats from Datanodes every 3s; Namenode state mantainer to check for heartbeats.

**You can read the detailed techincal design of the system [Techincal design doc](https://shared-goose-00a.notion.site/Whispering-Woods-20ee664bd91380deaff1d361c0ea8abf)**

## Installation & Setup
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [protoc](https://grpc.io/docs/protoc-installation/)
- [Docker](https://www.docker.com/)
- [jq](https://jqlang.org/download/)

### Setup
You can run this project on host directly or use docker to run it as cluster. Running on Host machine is convenient for testing purpose since you don't have to build container again and again.

In the Below section, instructions to run sytem are provided all the commands assume that you current working directory is root directory of this project until or unless explicitly specified.

Before running the system you need to setup logging and APM containers in docker you can refer to [logger setup](./logger/setup.md) File in order to do this.

**Running on host machine**

To run this system on host you need to run the bash script named **start.sh** this will take care of running the filebeat container as well. 
```
sh start.sh 
```
Simlary to stop the processes this system, you need to run other shell script named **stop.sh**
```
sh stop.sh
```

#### Running as cluster

To run this system as cluster we first need to create docker images for both Datanode and Namenode and Then we can use the scripts to start cluster. Client will still run on the host machine.

To avoid installing protoc inside builder container we will compile this on our host machine for this you need to install protoc and rust on your system and then simply run:
```
cargo build --release
```
This will build proto lib which needs protoc to compile Although this git repo contain the build proto lib in this repo but it will make sure any changes you made reflect in the code.

**Creating Namenode image**
```
docker build -f docker/Dockerfile.namenode -t gfs-namenode .
```

**Creating Datanode image**
```
docker build -f docker/Dockerfile.datanode -t gfs-datanode .
```

After creating both these images now we can you **start-cluster.sh** script to start the cluster passing datanode count as argument (by default 3 datanode and one namenode will be started).
```
sh start-cluster.sh 2
```

To Stop the started cluster you need first exit client using ctrl+c and then run **stop-cluster.sh** script
```
sh stop-cluster.sh
```

## Usage 
Whispering woods support three operations, each of these operation is initiated by writing command to client CLI. All these commands need host path relative to current working directory or abosulute path.

**Store file**: This operation is to store file from host to Whispering Woods. 
```
store SOURCE_FILE_PATH_ON_HOST NEW_FILE_NAME_ON_CLUSTER
```

**Fetch file**: This operation is to fetch file from Whispering Woods to host. 
```
fetch FILE_NAME_ON_CLUSTER TARGET_FILE_PATH_ON_HOST 
```

**Delete file**: This operation is to store file from host to Whispering Woods. 
```
delete FILE_NAME_ON_CLUSTER
```
## Dashboard
Whispering woods have a monitoring dashboard which display current cluster stats, available storage, active-inactive node. Files stored in clusters, chunks location and there health. To access this dashboard you need to use the **/dashboard/index.html** file once you login to the system using credentails dashboard will be acessible.

