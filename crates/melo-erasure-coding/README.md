# Melodot Erasure Coding

This is a core library for Melodot data availability sampling, providing functionalities for data encoding, data recovery, distributed generation, data validation, and more.

## Core Features

`melo-erasure-coding` aims to provide core functionalities for 2D erasure coding data availability techniques. Currently, we have completed the following features:

### Polynomial Commitment

We encode byte data into `k` `Blob`s, each `Blob` consisting of `n` elements of `32` bytes, with each element filled with `31` bytes of data from the original data. We treat the `Blob` as coefficients of an `n-1` degree polynomial and use polynomial commitment to generate a `KZGCommitment` for each `Blob`. By interpolation, we extend the `Blob` data to `2n` `Cell`s. Any `n` out of the `2n` elements can be used to recover the original `Blob`.

```text
|<-------- n -------->|<-------- 2n -------->|
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

-  Improve multi-threading performance
-  Optimize memory usage related to KZG setup
-  Implement column recovery

## References

- [Melodot whitepaper](https://www.notion.so/zerodao/Melodot-Data-Availability-Layer-Whitepaper-b72b1f3de81c40fc94a56763756ce34a?pvs=4)
- [kzg-rust](https://github.com/sifraitech/rust-kzg)
- [subspace](https://github.com/subspace/subspace)
- [substrate](https://github.com/paritytech/substrate)