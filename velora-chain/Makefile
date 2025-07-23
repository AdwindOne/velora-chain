# Makefile for velora-chain

CARGO := cargo
BINARY := velora
TARGET_DIR := target
PROFILE := release

# --------------------------
# Core Commands
# --------------------------

.PHONY: build
build:
	$(CARGO) build --$(PROFILE)

.PHONY: run
run:
	$(CARGO) run --$(PROFILE) --bin $(BINARY)

.PHONY: test
test:
	$(CARGO) test --workspace

.PHONY: lint
lint: format-check clippy

.PHONY: format-check
format-check:
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: format
format:
	$(CARGO) fmt --all

.PHONY: clean
clean:
	$(CARGO) clean

# --------------------------
# Devnet
# --------------------------

.PHONY: devnet
devnet: build
	@echo "Starting local devnet..."
	@bash ./scripts/devnet.sh

# --------------------------
# Docker
# --------------------------

.PHONY: docker-build
docker-build:
	@echo "Building Docker image..."
	@docker build -t velora/velora-chain:latest -f ../velora-docker/Dockerfile .

.PHONY: docker-run
docker-run:
	@echo "Running Docker container..."
	@docker run -it --rm velora/velora-chain:latest

# --------------------------
# Pre-commit
# --------------------------

.PHONY: hooks
hooks:
	pre-commit install

.PHONY: check-hooks
check-hooks:
	pre-commit run --all-files
