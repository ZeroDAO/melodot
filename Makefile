.PHONY: run-dev build-release build-default build-meloxt build-light purge-dev init test e2e run-light run-light-e2e bs build-farmer run-farmer run-overtrue build-node

run-light-dev: init
	./target/release/melodot-light --dev-mode

run-light-e2e: init
	./target/release/melodot-light-e2e

run-dev:
	./target/release/melodot-node --dev --ws-external

run-overtrue:
	./target/release/melodot-node --overtrue --ws-external

run-farmer:
	./target/release/melodot-farmer

build-release:
	cargo build --release

build-default:
	cargo build --release -p melodot-node -p melodot-runtime

build-node:
	cargo build --release -p melodot-node

build-meloxt:
	cargo build --release -p meloxt

build-light:
	cargo build --release -p melodot-light-client -p melodot-light-client-e2e

build-farmer:
	cargo build --release -p melodot-farmer-client

purge-dev:
	./target/release/melodot-node purge-chain --dev

init:
	./scripts/init.sh

test: init
	SKIP_WASM_BUILD=1 cargo test --release --all

e2e:
	./target/release/e2e

bs:
	cargo build --release -p melodot-node -p melodot-runtime --features runtime-benchmarks
