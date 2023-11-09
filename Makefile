.PHONY: run-dev build-release build-default build-meloxt build-light purge-dev init test e2e run-light run-light-e2e

run-light-dev: init
	./target/release/melodot-light --dev-mode

run-light-e2e: init
	./target/release/melodot-light-e2e

run-dev:
	./target/release/melodot-node --dev --ws-external

build-release:
	cargo build --release

build-default:
	cargo build --release -p melodot-node -p melodot-runtime

build-meloxt:
	cargo build --release -p meloxt

build-light:
	cargo build --release -p melodot-light-client -p melodot-light-client-e2e

purge-dev:
	./target/release/melodot-node purge-chain --dev

init:
	./scripts/init.sh

test: init
	SKIP_WASM_BUILD=1 cargo test --release --all

e2e:
	./target/release/e2e
