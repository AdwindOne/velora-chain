# --- Builder Stage ---
FROM rust:1.78-slim AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y build-essential

# Create a dummy project to cache dependencies
RUN USER=root cargo new --bin velora-chain
WORKDIR /build/velora-chain

# Copy over the manifests
COPY ./velora-chain/Cargo.toml ./Cargo.toml
COPY ./velora-chain/Cargo.lock ./Cargo.lock

# Build a dummy binary to cache dependencies
RUN cargo build --release --bin velora && rm src/main.rs target/release/velora

# Copy the actual source code
COPY ./velora-chain/src ./src
COPY ./velora-chain/crates ./crates

# Build the final binary
RUN cargo build --release --bin velora

# --- Final Stage ---
FROM debian:bullseye-slim

WORKDIR /velora

# Copy the binary from the builder stage
COPY --from=builder /build/velora-chain/target/release/velora /usr/local/bin/velora

# Copy the entrypoint script
COPY ./velora-docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Expose default ports
EXPOSE 30303
EXPOSE 8545

ENTRYPOINT ["/entrypoint.sh"]
CMD ["--help"]
