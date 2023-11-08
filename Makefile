.PHONY: run-dev build-release build-default build-meloxt build-light purge-dev init test e2e run-light run-light-e2e

run-light-dev:
	./target/release/melodot-light --dev-mode

run-light-e2e:
	./target/release/melodot-light-e2e

run-dev: init
	./target/release/melodot-node --dev --ws-external

build-release: init
	cargo build --release

build-default: init
	cargo build --release -p melodot-node -p melodot-runtime

build-meloxt: init
	cargo build --release -p meloxt

build-light: init
	cargo build --release -p melodot-light-client -p melodot-light-client-e2e

purge-dev: init
	./target/release/melodot-node purge-chain --dev

init:
	./scripts/init.sh

test: init
	SKIP_WASM_BUILD=1 cargo test --release --all

e2e: init
	./target/release/e2e
