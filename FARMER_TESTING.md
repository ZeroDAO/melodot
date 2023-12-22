# Farmer-client Testing guide

Melodot Farmer-client includes both unit tests and e2e tests, which can be executed locally or within a Docker container.

## ****Local Testing****

Ensure you have the necessary environment set up for Rust.

### ****Unit Tests****

Run all unit tests using the following command:

```bash
make test
```

### Building

We need to compile two projects: the melodot node and the Farmer-client. 

Known issue: Unable to compile under Mac environment, we will fix this issue later.

1. Build the melodot node using the following command, this may take some time:

```bash
make build-default
```

2. Compile the farmer-client using the following command:

```bash
make build-farmer
```

3. Compile the light client and e2e tests:

```bash
make build-light
```

### ****Launching the Development Network****

To initiate the development network, use the command below:

```bash
make run-dev
```

Once you observe the following output, it indicates that the development network is up and running, and blocks are being produced normally:

```bash
2023-11-09 21:36:31 Melodot Node    
2023-11-09 21:36:31 ✌️  version 0.0.1-1df3a1f033a    
2023-11-09 21:36:31 ❤️  by DKLee <xiuerdwy@gmail.com>, 2017-2023    
2023-11-09 21:36:31 📋 Chain specification: Development    
2023-11-09 21:36:31 🏷  Node name: accessible-cobweb-6597    
2023-11-09 21:36:31 👤 Role: AUTHORITY    
2023-11-09 21:36:31 💾 Database: RocksDb at /tmp/substrateJ6MN0y/chains/dev/db/full    
2023-11-09 21:36:31 ⛓  Native runtime: melodot-1 (melodot-1.tx1.au1)    
2023-11-09 21:36:32 [0] 💸 generated 1 npos voters, 1 from validators and 0 nominators    
2023-11-09 21:36:32 [0] 💸 generated 1 npos targets    
2023-11-09 21:36:33 🔨 Initializing Genesis block/state (state: 0x2538…5e46, header-hash: 0xac37…e2d5)    
2023-11-09 21:36:33 👴 Loading GRANDPA authority set from genesis on what appears to be first startup.    
2023-11-09 21:36:34 👶 Creating empty BABE epoch changes on what appears to be first startup.    
2023-11-09 21:36:34 Failed to register metrics: Duplicate metrics collector registration attempted    
2023-11-09 21:36:34 Using default protocol ID "sup" because none is configured in the chain specs    
2023-11-09 21:36:34 🏷  Local node identity is: 12D3KooWF8kNQjNivebHiCnTvkACt2SrNW6uEkJbyxWqzu1PAbVg    
2023-11-09 21:36:34 🚀 Starting transaction pool listener.    
2023-11-09 21:36:34 💻 Operating system: linux    
2023-11-09 21:36:34 💻 CPU architecture: x86_64    
2023-11-09 21:36:34 💻 Target environment: gnu    
2023-11-09 21:36:34 💻 CPU: Intel(R) Xeon(R) Platinum    
2023-11-09 21:36:34 💻 CPU cores: 4    
2023-11-09 21:36:34 💻 Memory: 7283MB    
2023-11-09 21:36:34 💻 Kernel: 5.15.0-79-generic    
2023-11-09 21:36:34 💻 Linux distribution: Ubuntu 22.04.3 LTS    
2023-11-09 21:36:34 💻 Virtual machine: yes    
2023-11-09 21:36:34 📦 Highest known block at #0    
2023-11-09 21:36:34 〽️ Prometheus exporter started at 127.0.0.1:9615    
2023-11-09 21:36:34 Running JSON-RPC HTTP server: addr=127.0.0.1:9933, allowed origins=["*"]    
2023-11-09 21:36:34 Running JSON-RPC WS server: addr=0.0.0.0:9944, allowed origins=["*"]    
2023-11-09 21:36:34 👶 Starting BABE Authorship worker    
2023-11-09 21:36:36 🙌 Starting consensus session on top of parent 0xac37c22f067cbea3a82a9952ed61a40a0a32eabb4a46fa96ebc230e63855e2d5    
2023-11-09 21:36:36 🎁 Prepared block for proposing at 1 (0 ms) [hash: 0x94cbc9ee49438476b9291ed9ea4dd722201bff89aaa61c803a6a484e218e3c82; parent_hash: 0xac37…e2d5; extrinsics (1): [0x6d27…d03d]]    
2023-11-09 21:36:36 🔖 Pre-sealed block for proposal at 1. Hash now 0x0b832715fa87a6e813606832ab364150830465fae6fd43f9b740763ba0eba75a, previously 0x94cbc9ee49438476b9291ed9ea4dd722201bff89aaa61c803a6a484e218e3c82.    
2023-11-09 21:36:36 👶 New epoch 0 launching at block 0x0b83…a75a (block slot 283256166 >= start slot 283256166).    
2023-11-09 21:36:36 👶 Next epoch starts at slot 283256766    
2023-11-09 21:36:36 😴 Block 1 has no blob    
2023-11-09 21:36:36 ✨ Imported #1 (0x0b83…a75a)    
2023-11-09 21:36:38 Accepting new connection 1/100
2023-11-09 21:36:39 discovered: 12D3KooW9wv5DVBvtUv9fy46PCkEDo9K1jyXzPS3SKiBbhW4rfty /ip4/172.19.0.1/tcp/4418    
2023-11-09 21:36:39 discovered: 12D3KooW9wv5DVBvtUv9fy46PCkEDo9K1jyXzPS3SKiBbhW4rfty /ip4/172.16.7.77/tcp/4418    
2023-11-09 21:36:39 discovered: 12D3KooW9wv5DVBvtUv9fy46PCkEDo9K1jyXzPS3SKiBbhW4rfty /ip4/172.17.0.1/tcp/4418    
2023-11-09 21:36:39 💤 Idle (0 peers), best: #1 (0x0b83…a75a), finalized #0 (0xac37…e2d5), ⬇ 0 ⬆ 0    
2023-11-09 21:36:42 🙌 Starting consensus session on top of parent 0x0b832715fa87a6e813606832ab364150830465fae6fd43f9b740763ba0eba75a    
2023-11-09 21:36:42 🎁 Prepared block for proposing at 2 (0 ms) [hash: 0x28b41376f2b51efd8def9083bffc3e5c5f98f15d266dfb1986172de9e09e26fc; parent_hash: 0x0b83…a75a; extrinsics (1): [0x57d3…a11a]]    
2023-11-09 21:36:42 🔖 Pre-sealed block for proposal at 2. Hash now 0xb0134001cfed9449650f3c8c6af26230dd2d6ac682b06391b5ec4187c4e365ff, previously 0x28b41376f2b51efd8def9083bffc3e5c5f98f15d266dfb1986172de9e09e26fc.    
2023-11-09 21:36:42 😴 Block 2 has no blob    
2023-11-09 21:36:42 ✨ Imported #2 (0xb013…65ff)    
2023-11-09 21:36:44 💤 Idle (0 peers), best: #2 (0xb013…65ff), finalized #0 (0xac37…e2d5), ⬇ 0 ⬆ 0    
2023-11-09 21:36:48 🙌 Starting consensus session on top of parent 0xb0134001cfed9449650f3c8c6af26230dd2d6ac682b06391b5ec4187c4e365ff    
2023-11-09 21:36:48 🎁 Prepared block for proposing at 3 (0 ms) [hash: 0x73880cfd4f67132321ac78829d30389869c233e8ba528c6b19627ef4b7db8c48; parent_hash: 0xb013…65ff; extrinsics (1): [0xb35b…16f5]]   
```

### ****Running Farmer-clent****

Open a new terminal and execute the following command to run the farmer-client:

```bash
make run-farmer
```

Once you observe the following output, it indicates that the farmer-client is up and running:

```bash
 INFO 🚀 Melodot Farmer Client starting up    
 INFO creating instance on iface 172.16.7.77    
 INFO creating instance on iface 172.19.0.1    
 INFO creating instance on iface 172.17.0.1    
 INFO 🌐 Subscribed to best block headers    
 INFO ⚓ Received best block header #0    
 INFO ⏭️  No data in block #0    
 INFO discovered: 12D3KooWDA8b1nVXeU22coo6cy217DUucgF5zcTRfa7DqXRnSE9L /ip4/172.16.7.77/tcp/4417    
 INFO discovered: 12D3KooWDA8b1nVXeU22coo6cy217DUucgF5zcTRfa7DqXRnSE9L /ip4/172.17.0.1/tcp/4417    
 INFO discovered: 12D3KooWDA8b1nVXeU22coo6cy217DUucgF5zcTRfa7DqXRnSE9L /ip4/172.19.0.1/tcp/4417    
 INFO ⚓ Received best block header #1    
 INFO ⏭️  No data in block #1    
```

If you encounter a network error, it may be because the melodot-node has not yet initialized the network. You just need to wait a few seconds and then run the farmer-client again.

### ****Running e2e Tests****

Ensure the test network and farmer-client are running. Open a new terminal and execute the e2e tests using the following command:

```bash
make e2e
```

This will run the end-to-end (e2e) tests for melodot-node, including data upload. You will see the following output:

```bash
Running example: header
   Compiling melodot-runtime v0.0.1 (/root/melodot/runtime)
   Compiling melo-das-rpc v0.0.1 (/root/melodot/crates/das-rpc)
   Compiling meloxt v0.0.1 (/root/melodot/crates/meloxt)
    Finished release [optimized] target(s) in 3m 24s
     Running `target/release/examples/header`
 INFO 🌟 Start get header    
 INFO ✅ Success Current block_num: 35    
 INFO 💯 All success : Header    
Running example: register_app
   Compiling meloxt v0.0.1 (/root/melodot/crates/meloxt)
    Finished release [optimized] target(s) in 19.50s
     Running `target/release/examples/register_app`
 INFO 🌟 Start register app     
 INFO ✅ Success Application created, block hash: 0x8032…2426    
 INFO 💯 All success : Register app    
Running example: submit_blob_tx
   Compiling meloxt v0.0.1 (/root/melodot/crates/meloxt)
    Finished release [optimized] target(s) in 15.96s
     Running `target/release/examples/submit_blob_tx`
 INFO 🌟 Start submit blob tx    
 INFO ✅ Success: Commitments bytes: [171, 26, 161, 31, 215, 245, 79, 117, 106, 34, 232, 99, 28, 0, 229, 164, 6, 136, 85, 40, 22, 77, 143, 120, 175, 238, 104, 50, 7, 50, 218, 226, 191, 226, 69, 52, 179, 38, 228, 236, 179, 122, 101, 10, 130, 105, 126, 189]    
 INFO ✅ Success: Data submited, tx_hash: 0x6db80e4e172677cbab41c5891035b8bdef538a7b05d7f24f2e0b43197dc5bcc8    
 INFO ⏳ Data not verified yet, current block number: 45    
 INFO ⏳ Data not verified yet, current block number: 46    
 INFO ✅ Success Data should have been verified by the validators at: 47    
 INFO 💯 All success : Submit blob tx    
Running example: submit_data
   Compiling meloxt v0.0.1 (/root/melodot/crates/meloxt)
    Finished release [optimized] target(s) in 15.55s
     Running `target/release/examples/submit_data`
 INFO 🌟 Start submit data    
 INFO ✅ Success: Data submited, block hash: 0x4c49…41fe    
 INFO 💯 All success : Submit data    
Running example: submit_invalid_blob_tx
   Compiling meloxt v0.0.1 (/root/melodot/crates/meloxt)
    Finished release [optimized] target(s) in 14.24s
     Running `target/release/examples/submit_invalid_blob_tx`
 INFO 🌟 Start submit invalid blob tx    
 INFO ✅ Success: Commitments bytes: [166, 182, 73, 97, 133, 21, 86, 237, 128, 28, 102, 130, 250, 210, 22, 184, 0, 123, 104, 160, 30, 122, 92, 38, 197, 190, 67, 98, 134, 77, 243, 183, 214, 132, 242, 125, 137, 179, 229, 153, 160, 233, 193, 101, 224, 104, 102, 143]    
 INFO ✅ Success: Submit InvalidExtrinsic, tx failed with code: 10002    
 INFO ✅ Success: Submit InvalidBytesLen, tx failed with code: 10005    
 INFO ✅ Success: Submit invalid commitments, tx failed: "Data verification failed. Please check your data and try again."    
 INFO ✅ Success: Submit invalid proofs, tx failed: "Data verification failed. Please check your data and try again."    
 INFO 💯 All success : Submit invalid blob tx
```

At this point, the farmer-client should receive the data and save it locally. If successful, you should see the following output in the farmer-client:

```
INFO 🚩 Data rows num: 1    
 INFO 💾 Data saved successfully    
 INFO ⚓ Received best block header #48    
 INFO ⏭️  No data in block #48    
 INFO ⚓ Received best block header #49    
 INFO ⏭️  No data in block #49    
 INFO ⚓ Received best block header #50    
 INFO ⏭️  No data in block #50    
 INFO ⚓ Received best block header #51    
 INFO 🚩 Data rows num: 1    
 INFO 💾 Data saved successfully    
 INFO ⚓ Received best block header #52    
 INFO ⏭️  No data in block #52    
 INFO ⚓ Received best block header #53    
 INFO ⏭️  No data in block #53    
 INFO ⚓ Received best block header #54    
 INFO ⏭️  No data in block #54    
 INFO ⚓ Received best block header #55    
 INFO ⏭️  No data in block #55    
 INFO ⚓ Received best block header #56    
 INFO 🚩 Data rows num: 1    
 INFO 💾 Data saved successfully  
```

## **Using Docker**

First, install Docker and Docker Compose.

You need to run the following commands in the root directory of **`melodot`**:

```bash
./scripts/docker_run.sh
```

This command will build a Docker image and start a Docker container. Within the container, you can carry out the same steps as in the previous section for unit testing, building, and running the development network.

You can then open a new Docker terminal with the following command, running the farmer-client. 

```bash
./scripts/docker_run.sh new
```

Finally, open another new Docker terminal to run the e2e tests:

```bash
make run-farmer
```

Then, open another new Docker terminal to run the e2e tests:

```bash
make e2e
```
