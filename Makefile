.PHONY: run-dev
run-dev:
	./target/release/melodot-node --dev --ws-external

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: build-default
build-default:
	cargo build --release -p node -p runtime

.PHONY: build-meloxt
build-meloxt:
	cargo build --release -p meloxt

.PHONY: build-light
build-light:
	cargo build --release -p melodot-light-client

.PHONY: purge-dev
purge-dev:
	./target/release/melodot-node purge-chain --dev

.PHONY: init
init:
	./scripts/init.sh

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --release --all

.PHONY: e2e
e2e:
	./target/release/e2e
