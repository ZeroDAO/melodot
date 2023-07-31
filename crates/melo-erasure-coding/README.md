# Melodot Erasure Coding

This is a core library for Melodot data availability sampling, providing functionalities for data encoding, data recovery, distributed generation, data validation, and more.

## Core Features

`melo-erasure-coding` aims to provide core functionalities for 2D erasure coding data availability techniques. Currently, we have completed the following features:

### Polynomial Commitment

We encode byte data into `k` `Blob`s, each `Blob` consisting of `n` elements of `32` bytes, with each element filled with `31` bytes of data from the original data. We treat the `Blob` as coefficients of an `n-1` degree polynomial and use polynomial commitment to generate a `KZGCommitment` for each `Blob`. By interpolation, we extend the `Blob` data to `2n` `Cell`s. Any `n` out of the `2n` elements can be used to recover the original `Blob`.

```text
|<------1...n ------->|<------n+1...2n ----->|
+-------+     +-------+-------+     +--------+ +------------+
|  c_1  | ... |  c_n  | c_n+1 | ... |  c_2n  | | commitment |
+-------+     +-------+-------+     +--------+ +------------+
```

### Segment

As sampling directly at the `Cell` level with a `KZGProof` of size `48` bytes for each `Cell` is inefficient and impractical, we group a fixed number (`Segment::SIZE`) of `Cell`s into a `Segment`, and these data blocks share the same `KZGProof`. Ultimately, we sample at the `Segment` level.

```text
           Segment
+------+------+------+------+
| cell | cell | cell | cell |
+------+------+------+------+
|           proof           |
+---------------------------+
```

### Distributed Generation

Distributed generation eliminates the need for a powerful producer node and distributes data and computation across different nodes and roles, enhancing the decentralization of the system.

Nodes obtain `k` `KZGCommitment`s for the original columns from block headers, treat them as Lagrange form (i.e., points of the polynomial), and then extend to `2k` points to obtain `KZGCommitment`s for rows `k + 1` to `2k`.

At the `Segment` level, nodes can directly generate extended columns, including data itself and `KZGProof`, based on the sampled original column data. This utilizes the homomorphic property of KZG commitments, allowing nodes to immediately verify the correctness of the extended data. As a result, the need for recomputing polynomial commitments and proofs is avoided, and the data is extended to a 2D pattern. The crate's `extend_segments_col` implements this functionality, taking `Segments` from the first `k` columns and returning the extended `Segments`, including `KZGProof`.

### Data Recovery

Currently, we support row recovery. You can use the `recover` method in the `erasures-coding` module to recover row data in units of `BlsScalar`, or use the `recover_poly` method to recover polynomials. It is also possible to recover `Segments` using the `recovery_row_from_segments` method.

## Usage

To utilize most functionalities of this crate, it is essential to construct a `KZGSetting` first, which facilitates subsequent data encoding and recovery. We provide a default embedded setting for convenience:

```rust
// Assuming you have already imported the necessary modules or structs.

// Create a default embedded KZGSetting for convenience.
let kzg = KZG::default_embedded();

// Prepare the data to be committed.
let bytes: Vec<[u8; 31]> = vec![[255; 31]; 64];

// Commit the data using the KZGSetting.
let commitment = Blob::try_from_bytes(&bytes, 64)?.commit(&kzg);
```

To construct a custom `KZGSetting`, you can use the `bytes_to_kzg_settings` function available in the `kzg` module. This function requires passing precomputed data. Additionally, you have the option to generate data of various lengths from Ethereum-style trusted setups located in the `melodot` root directory using the following command:

```bash
./scripts/process_data.sh 4096
```

By changing the value `4096`, you can generate data of different lengths. The available options are: `["4096" "8192" "16384" "32768"]`.

Next you can build `KZGSetting` using `embedded_kzg_settings`.

```rust
// Assuming you have already imported the necessary modules or structs.

const EMBEDDED_KZG_SETTINGS_BYTES: &[u8] = include_bytes!("../scripts/eth-public-parameters-4096.bin");

// Create a KZGSetting from the precomputed data.
let kzg = KZG::embedded_kzg_settings(EMBEDDED_KZG_SETTINGS_BYTES);

```

## Testing

To run unit tests, you can use the following command:

```bash
cargo test
```

## Docker

First, install Docker and Docker Compose.

You need to run the following commands in the root directory of `melodot`:

```bash
./scripts/docker_run.sh
```

This command will build a Docker image and start a Docker container. You can run `cargo` commands inside the container to perform unit tests and build the crate.

```bash
cd crates/melo-erasure-coding
cargo test
cargo build
```

Alternatively, you can directly use the following commands to perform unit tests and build the crate:

```bash
# test melo-erasure-coding
./scripts/docker_run.sh test melo-erasure-coding
# build melo-erasure-coding
./scripts/docker_run.sh build melo-erasure-coding
```

## TODO List

- [ ] Improve multi-threading performance
- [ ] Optimize memory usage related to KZG setup
- [ ] Implement column recovery

## References

- [Melodot whitepaper](https://www.notion.so/zerodao/Melodot-Data-Availability-Layer-Whitepaper-b72b1f3de81c40fc94a56763756ce34a?pvs=4)
- [kzg-rust](https://github.com/sifraitech/rust-kzg)
- [subspace](https://github.com/subspace/subspace)
- [substrate](https://github.com/paritytech/substrate)