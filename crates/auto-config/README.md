# Melo Auto Config

## Description

`melo-auto-config` is a Rust library providing a procedural macro (`proc_macro`) for automatically generating configuration implementations for Substrate pallets. This macro supports optional generation of `RuntimeEvent`, `WeightInfo`, and `Currency` types, while also auto-detecting the pallet name. The goal is to simplify the repetitive development process in configuring runtime and we will continue to add more automatic configurations.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
melo-auto-config = "0.9.42"
```

**Note:** The versioning of `melo-auto-config` is designed to match Polkadot's versioning scheme. For example, if you are using `polkadot-v0.9.42`, you should use `melo-auto-config = "0.9.42"` to ensure compatibility.

## Usage

Use the `#[auto_config]` macro in your Substrate pallet's configuration implementation:

```rust
use melo_auto_config::auto_config;

#[auto_config]
impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    // other types...
}
```

### Attributes

- `skip_event`: Skips the generation of `RuntimeEvent` type
- `skip_weight`: Skips the generation of `WeightInfo` type
- `include_currency`: Includes the generation of `Currency` type

Example:

```rust
#[auto_config(skip_event, include_currency)]
impl pallet_balances::Config for Runtime {
    // ...
}
```

This will generate an implementation that includes the `Currency` type but not the `RuntimeEvent` type.

## License

Apache-2.0
