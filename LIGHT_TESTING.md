# Light-client Testing guide

Melodot Light-client includes both unit tests and e2e tests, which can be executed locally or within a Docker container.

## ****Local Testing****

Ensure you have the necessary environment set up for Rust.

### ****Unit Tests****

Run all unit tests using the following command:

```bash
make test
```

### Building

We need to compile two projects: the melodot node and the light-client. 

Known issue: Unable to compile under Mac environment, we will fix this issue later.

1. Build the melodot node using the following command, this may take some time:

```bash
make build-default
```

2. Compile the light-client using the following command:

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

### ****Running Light-clent****

Open a new terminal and execute the following command to run the light-client:

```bash
make run-light-dev
```

Once you observe the following output, it indicates that the light-client is up and running:

```bash
 INFO 🚀 Melodot Light Client starting up    
 INFO 👂 RPC server started at: 127.0.0.1:4177    
 INFO creating instance on iface 172.16.7.77    
 INFO creating instance on iface 172.19.0.1    
 INFO creating instance on iface 172.17.0.1    
 INFO 🌐 Subscribed to finalized block headers    
 INFO ✅ Received finalized block header #0    
 INFO discovered: 12D3KooWKdiBnPzEWuXEk6nwJvmxXt7QrkC71eCMBEXP1jBQiYgf /ip4/172.17.0.1/tcp/4417    
 INFO discovered: 12D3KooWKdiBnPzEWuXEk6nwJvmxXt7QrkC71eCMBEXP1jBQiYgf /ip4/172.16.7.77/tcp/4417    
 INFO discovered: 12D3KooWKdiBnPzEWuXEk6nwJvmxXt7QrkC71eCMBEXP1jBQiYgf /ip4/172.19.0.1/tcp/4417    
 INFO connection{remote_addr=127.0.0.1:38114 conn_id=0}: Accepting new connection 1/100
 INFO ✅ Received finalized block header #1    
 INFO ✅ Received finalized block header #2    
 INFO ✅ Received finalized block header #3
```

### ****Running e2e Tests****

Ensure the test network and light-client are running. Open a new terminal and execute the e2e tests using the following command:

```bash
make run-light-e2e
```

This will start the e2e tests, submitting data transactions to the node, and obtaining light-client sample data when the block is finalized to verify if the data is indeed available. Afterward, it will submit another data transaction and delete most of the data from the network after it is successfully included in the block, to verify if the light-client can correctly identify the unavailability of data through sampling. If you see the following output, it indicates all tests have been successfully completed:

```bash
 INFO 🚀 Melodot Light Client e2e starting up    
 INFO 🌟 Start: Running data availability    
 INFO ✅ Success Application created, block hash: 0xb3c3…d118    
 INFO ✅ Success: Data submitted, tx_hash: 0x986f0fea84a91a7b5eb78228df50870580b01895979dd1acb64a4808928ddeab    
 INFO ⏳ Data not verified yet, current block number: 6    
 INFO ⏳ Data not verified yet, current block number: 7    
 INFO ✅ Success Data should have been verified by the validators at: 8    
 INFO ⏳ Data not finalized yet, current block number: 5    
 INFO ⏳ Data not finalized yet, current block number: 6    
 INFO ⏳ Data not finalized yet, current block number: 7    
 INFO ✅ Success Data finalized at block: 8    
 INFO ⏳ Wait for the sampling to complete.    
 INFO ✅ Success: Block confidence is above 99.99%: 999985    
 INFO 💯 All success : Module data_availability    
 INFO 🌟 Start: Running data_unavailable    
 INFO ✅ Success: Data submitted, tx_hash: 0x9d1818b12bfcdfa34ae9943903f7bb2044ab05e8bdb062a95a303811b26eb0b5    
 INFO ⏳ Data not verified yet, current block number: 10    
 INFO ⏳ Data not verified yet, current block number: 11    
 INFO ✅ Success Data should have been verified by the validators at: 12    
 INFO ⏳: Waiting for data to be propagated across the network.    
 INFO ✅ Success: 75% of data has been deleted    
 INFO ⏳ Data not finalized yet, current block number: 10    
 INFO ⏳ Data not finalized yet, current block number: 11    
 INFO ✅ Success Data finalized at block: 12    
 INFO ⏳: Wait for the sampling to complete.    
 INFO ✅ Success: Block confidence is less than 99.99%: 750000    
 INFO 💯 All success: Module data_unavailable
```

## **Using Docker**

First, install Docker and Docker Compose.

You need to run the following commands in the root directory of **`melodot`**:

```bash
./scripts/docker_run.sh
```

This command will build a Docker image and start a Docker container. Within the container, you can carry out the same steps as in the previous section for unit testing, building, and running the development network.

You can then open a new Docker terminal with the following command, running the light-client. 

```bash
./scripts/docker_run.sh new
```

Finally, open another new Docker terminal to run the e2e tests:

```bash
make run-light-e2e
```
