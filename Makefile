.PHONY: run-dev
run-dev:
	./target/release/melodot-node --dev --ws-external

.PHONY: build-release
build-release:
	cargo build --release

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

.PHONY: docker
docker:
	./scripts/docker_run.sh

.PHONY: docker-run-dev
docker-run-dev:
	./scripts/docker_run.sh ./target/release/melodot-node --dev --ws-external

.PHONY: docker-e2e
docker-e2e:
	./scripts/docker_run.sh ./target/release/e2e