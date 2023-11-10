# Melodot Light Client 

## 1. Introduction

The Melodot network's light client is currently operational in the development network. Light nodes can access the finalized block headers from the Melodot network through RPC and validate the data's authenticity by sampling.

## 2. Operating Mechanism

Melodot light nodes employ sampling to assess data availability. They do not need to trust any nodes or download the entire dataset.

Light nodes initially acquire the latest finalized block header through a full node. They then construct sample IDs using the metadata in the block header. Subsequently, these random samples are sourced from a separate data network and validated against the KZG commitments in the block header.

## 3. Building 

Building from source
    
```bash
git clone git@github.com:ZeroDAO/melodot.git
cd melodot
make build-light
```

## 4. Running

First, ensure that the Melodot development network is running, then execute the light node to join the development network

```bash
make run-light-dev
```

## 5. RPC Interface

### POST /das_blockConfidence

Retrieve the confidence level for a block hash, returning a confidence level measured in parts per million.
    
```bash
curl --data '{"method":"das_blockConfidence","params":["0xc964c3636fdf33bcc4ccc7ad854b32862e02ec50f02a00ba43f0b02c4fbb67e3"],"id":1,"jsonrpc":"2.0"}' -H "Content-Type: application/json" -X POST localhost:4177
```

***Responses***

```json
{"jsonrpc":"2.0","result":937500,"id":1}
```

### POST /das_isAvailable

Check if the data for a block hash is available. A return of null indicates no data for that block.

```bash
curl --data '{"method":"das_isAvailable","params":["0xc964c3636fdf33bcc4ccc7ad854b32862e02ec50f02a00ba43f0b02c4fbb67e3"],"id":1,"jsonrpc":"2.0"}' -H "Content-Type: application/json" -X POST localhost:4177
```

***Responses***

```json
{"jsonrpc":"2.0","result":false,"id":1}
```

## 5. Planned Features

- [ ] Validate finalized block headers
- [ ] Send transactions and push data to the network
- [ ] Retrieve application data using nonce and app_id
- [ ] Access data by rows and columns
- [ ] Support data reconstruction