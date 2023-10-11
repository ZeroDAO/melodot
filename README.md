<div align="center">
  <img src="https://pic.tom24h.com/melo/Melodot-min.png" width="100%">
</div>

## 1. Introduction

Here's the translation:

Melodot is designed as a data availability layer for GB-level data throughput, featuring:

1. Utilizes KZG commitments to ensure data is correctly encoded.
2. Incorporates "farmers" so the system doesn't rely on the "minimal honest sampler" assumption.
3. Distributively produced, achieving complete decentralization.
4. Distributive data storage for availability.
5. Validators act more like light clients, making it easier for sharding.

Melodot is actively under development; modules and interfaces are subject to significant changes. More details can be found in the [documentation](https://docs.melodot.io).

<div align="center">
    <img src="https://github.com/ZeroDAO/www.ourspace.network/blob/main/src/assets/images/w3f.svg" width="500">
</div>

## 2. Building

### Setup rust

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

You will also need to install the following packages:

mac

```bash
brew install cmake pkg-config openssl git llvm
```

Linux

```bash
sudo apt install cmake pkg-config libssl-dev git clang libclang-dev protobuf-compiler
```

More：Melodot is based on Substrate, for more information please go to [Substrate](https://docs.substrate.io/install/).

### Build

```bash
make build-release
```

## 3. Run

You can start a development chain with:

```bash
make run-dev
```

## 4. Development

### Test All

Use the following command to run all tests:

```bash
make test
```

You can learn more detailed testing methods from the [testing guide](./TESTING.md).

## 5. Docker

Start a Docker container:

```bash
./scripts/docker_run.sh
```

You can learn more about Docker examples from the [testing guide](./TESTING.md).

## Reference

- [Melodot Whitepaper](https://zerodao.notion.site/Melodot-Data-Availability-Layer-Whitepaper-b72b1f3de81c40fc94a56763756ce34a?pvs=4)
- [substrate](https://github.com/paritytech/substrate)
- [kzg-rust](https://github.com/sifraitech/rust-kzg)
- [subspace](https://github.com/subspace/subspace)

