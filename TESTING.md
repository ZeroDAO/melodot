# Testing guide

Melodot includes both unit tests and e2e tests, which can be executed locally or within a Docker container.

## ****Local Testing****

Ensure you have the necessary environment set up for Rust.

### ****Unit Tests****

Run all unit tests using the following command:

```bash
make test
```

### Building

build using the command below; this might take a significant amount of time:

```bash
make build-release
```

### ****Launching the Development Network****

To initiate the development network, use the command below:

```bash
make run-dev
```

Once you observe the following output, it indicates that the development network is up and running, and blocks are being produced normally:

```bash
2023-10-03 15:24:15 Low open file descriptor limit configured for the process. Current value: 4096, recommended value: 10000.
2023-10-03 15:24:15 Substrate Node
2023-10-03 15:24:15 ✌️  version 0.0.1-82ce44a3195
2023-10-03 15:24:15 ❤️  by DKLee <xiuerdwy@gmail.com>, 2017-2023
2023-10-03 15:24:15 📋 Chain specification: Development
2023-10-03 15:24:15 🏷  Node name: physical-rabbits-2439
2023-10-03 15:24:15 👤 Role: AUTHORITY
2023-10-03 15:24:15 💾 Database: RocksDb at /tmp/substrateAanHqJ/chains/dev/db/full
2023-10-03 15:24:15 ⛓  Native runtime: melodot-1 (melodot-1.tx1.au1)
2023-10-03 15:24:15 [0] 💸 generated 1 npos voters, 1 from validators and 0 nominators
2023-10-03 15:24:15 [0] 💸 generated 1 npos targets
2023-10-03 15:24:17 🔨 Initializing Genesis block/state (state: 0x6f53…5273, header-hash: 0x93f5…209a)
2023-10-03 15:24:17 👴 Loading GRANDPA authority set from genesis on what appears to be first startup.
2023-10-03 15:24:19 👶 Creating empty BABE epoch changes on what appears to be first startup.
2023-10-03 15:24:19 Using default protocol ID "sup" because none is configured in the chain specs
2023-10-03 15:24:19 🏷  Local node identity is: 12D3KooWPeEmREcwTij74dDnAG6NRT1H5UpdwZwRuzs5nUBi2u38
2023-10-03 15:24:19 Starting transaction pool listener.
2023-10-03 15:24:19 💻 Operating system: linux
2023-10-03 15:24:19 💻 CPU architecture: x86_64
2023-10-03 15:24:19 💻 Target environment: gnu
2023-10-03 15:24:19 💻 CPU: Intel(R) Core(TM) i7-4790K CPU @ 4.00GHz
2023-10-03 15:24:19 💻 CPU cores: 4
2023-10-03 15:24:19 💻 Memory: 12670MB
2023-10-03 15:24:19 💻 Kernel: 5.10.60.1-microsoft-standard-WSL2
2023-10-03 15:24:19 💻 Linux distribution: Ubuntu 22.04.2 LTS
2023-10-03 15:24:19 💻 Virtual machine: yes
2023-10-03 15:24:19 📦 Highest known block at #0
2023-10-03 15:24:19 Running JSON-RPC HTTP server: addr=127.0.0.1:9933, allowed origins=["*"]
2023-10-03 15:24:19 Running JSON-RPC WS server: addr=0.0.0.0:9944, allowed origins=["*"]
2023-10-03 15:24:19 〽️ Prometheus exporter started at 127.0.0.1:9615
2023-10-03 15:24:19 👶 Starting BABE Authorship worker
2023-10-03 15:24:24 🙌 Starting consensus session on top of parent 0x93f5c0a8fae6ee6cb5db6844e42195eda9d1371967ee17fded1897cc776f209a
2023-10-03 15:24:24 🎁 Prepared block for proposing at 1 (0 ms) [hash: 0x10e18d51524811bdfd16c77c07394540ba1dd696cfa7b982b8941640364edcaf; parent_hash: 0x93f5…209a; extrinsics (1): [0x07a5…1968]]
2023-10-03 15:24:24 🔖 Pre-sealed block for proposal at 1. Hash now 0x1a164e95126273b65d263290f461b530a097f4deccc2dc477efcd4a313250ed5, previously 0x10e18d51524811bdfd16c77c07394540ba1dd696cfa7b982b8941640364edcaf.
2023-10-03 15:24:24 👶 New epoch 0 launching at block 0x1a16…0ed5 (block slot 282719644 >= start slot 282719644).
2023-10-03 15:24:24 👶 Next epoch starts at slot 282720244
2023-10-03 15:24:24 ✨ Imported #1 (0x1a16…0ed5)
2023-10-03 15:24:24 💤 Idle (0 peers), best: #1 (0x1a16…0ed5), finalized #0 (0x93f5…209a), ⬇ 0 ⬆ 0
2023-10-03 15:24:29 💤 Idle (0 peers), best: #1 (0x1a16…0ed5), finalized #0 (0x93f5…209a), ⬇ 0 ⬆ 0
2023-10-03 15:24:30 🙌 Starting consensus session on top of parent 0x1a164e95126273b65d263290f461b530a097f4deccc2dc477efcd4a313250ed5
2023-10-03 15:24:30 🎁 Prepared block for proposing at 2 (0 ms) [hash: 0xc17cac729a20694bd671abb55a242a1733d1ebcd0507856c88a7d8b7eab281d9; parent_hash: 0x1a16…0ed5; extrinsics (1): [0x48d2…edda]]
2023-10-03 15:24:30 🔖 Pre-sealed block for proposal at 2. Hash now 0xcd408d53decedf89527061e6a890b6fa4ab5980f3f6e64a589bd49a80f0deeca, previously 0xc17cac729a20694bd671abb55a242a1733d1ebcd0507856c88a7d8b7eab281d9.
2023-10-03 15:24:30 ✨ Imported #2 (0xcd40…eeca)
2023-10-03 15:24:34 💤 Idle (0 peers), best: #2 (0xcd40…eeca), finalized #0 (0x93f5…209a), ⬇ 0 ⬆ 0
```

### ****Running e2e Tests****

Ensure that the development network from the previous step is running properly. Open a new terminal and execute the e2e tests using the following command:

```bash
make e2e
```

The initial run might take a considerable amount of time. Once completed, you should see the following output, indicating that all tests have been successfully completed:

```bash
Running example: header
    Finished release [optimized] target(s) in 1.20s
     Running `target/release/examples/header`
2023-10-03 15:28:05,802 INFO [header] 🌟 Start get header
2023-10-03 15:28:05,833 INFO [header] ✅ Success Current block_num: 37
2023-10-03 15:28:05,833 INFO [header] 💯 All success : Header
Running example: register_app
    Finished release [optimized] target(s) in 0.63s
     Running `target/release/examples/register_app`
2023-10-03 15:28:06,588 INFO [register_app] 🌟 Start register app
2023-10-03 15:28:24,944 INFO [register_app] ✅ Success Application created, block hash: 0x9388…73e8
2023-10-03 15:28:24,944 INFO [register_app] 💯 All success : Register app
Running example: submit_blob_tx
    Finished release [optimized] target(s) in 0.63s
     Running `target/release/examples/submit_blob_tx`
2023-10-03 15:28:25,698 INFO [submit_blob_tx] 🌟 Start submit blob tx
2023-10-03 15:28:25,814 INFO [submit_blob_tx] ✅ Success: Commitments bytes: [164, 244, 49, 244, 158, 40, 64, 171, 190, 65, 165, 242, 150, 187, 96, 227, 82, 40, 209, 203, 140, 161, 83, 130, 134, 216, 59, 95, 166, 245, 167, 198, 71, 254, 155, 165, 70, 102, 68, 183, 236, 117, 194, 182, 61, 169, 16, 117]
2023-10-03 15:28:25,904 INFO [submit_blob_tx] ✅ Success: Data submited, tx_hash: 0x0bc8249ccd679bf42bf66f98c8acd9403cf7bb4fad807af5b8fdc1b60a2a2ee0
2023-10-03 15:28:25,904 INFO [submit_blob_tx] ⏳ Data not verified yet, current block number: 41
2023-10-03 15:28:30,006 INFO [submit_blob_tx] ⏳ Data not verified yet, current block number: 42
2023-10-03 15:28:36,005 INFO [submit_blob_tx] ✅ Success Data should have been verified by the validators at: 43
2023-10-03 15:28:36,005 INFO [submit_blob_tx] 💯 All success : Submit blob tx
Running example: submit_data
    Finished release [optimized] target(s) in 0.59s
     Running `target/release/examples/submit_data`
2023-10-03 15:28:36,711 INFO [submit_data] 🌟 Start submit data
2023-10-03 15:28:55,627 INFO [submit_data] ✅ Success: Data submited, block hash: 0x72d4…3568
2023-10-03 15:28:55,627 INFO [submit_data] 💯 All success : Submit data
Running example: submit_invalid_blob_tx
    Finished release [optimized] target(s) in 0.60s
     Running `target/release/examples/submit_invalid_blob_tx`
2023-10-03 15:28:56,331 INFO [submit_invalid_blob_tx] 🌟 Start submit invalid blob tx
2023-10-03 15:28:56,447 INFO [submit_invalid_blob_tx] ✅ Success: Commitments bytes: [142, 29, 116, 231, 181, 125, 122, 228, 235, 144, 4, 84, 114, 146, 93, 43, 165, 237, 38, 70, 103, 238, 88, 210, 171, 10, 99, 215, 231, 159, 193, 238, 8, 61, 79, 183, 15, 24, 100, 22, 125, 247, 96, 6, 96, 99, 86, 188]
2023-10-03 15:28:56,449 INFO [submit_invalid_blob_tx] ✅ Success: Submit InvalidBlob, tx failed with code: 10005
2023-10-03 15:28:56,449 INFO [submit_invalid_blob_tx] ✅ Success: Submit InvalidExtrinsic, tx failed with code: 10002
2023-10-03 15:28:56,451 INFO [submit_invalid_blob_tx] ✅ Success: Submit InvalidDataHash, tx failed with code: 10005
2023-10-03 15:28:56,452 INFO [submit_invalid_blob_tx] ✅ Success: Submit InvalidBytesLen, tx failed with code: 10005
2023-10-03 15:28:56,542 INFO [submit_invalid_blob_tx] ✅ Success: Submit invalid commitments, tx failed: "Data verification failed. Please check your data and try again."
2023-10-03 15:29:00,105 INFO [submit_invalid_blob_tx] ✅ Success: Submit invalid proofs, tx failed: "Data verification failed. Please check your data and try again."
2023-10-03 15:29:06,007 INFO [submit_invalid_blob_tx] 💯 All success : Submit invalid blob tx
```

## **Using Docker**

First, install Docker and Docker Compose.

You need to run the following commands in the root directory of **`melodot`**:

```bash
./scripts/docker_run.sh
```

This command will build a Docker image and start a Docker container. Within the container, you can carry out the same steps as in the previous section for unit testing, building, and running the development network.

You can then open a new Docker terminal with the following command:

```bash
./scripts/docker_run.sh new
```

While ensuring the development network is running properly, execute the following command in the new terminal to run the e2e tests:

```bash
make e2e
```
