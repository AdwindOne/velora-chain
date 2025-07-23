# Velora: A Modular, High-Performance Rust Blockchain

**Velora** is a next-generation Layer 1 blockchain built with Rust, designed for performance, modularity, and EVM compatibility. It aims to provide a robust platform for decentralized applications by leveraging cutting-edge technologies like `revm`, parallel execution, and a pluggable consensus mechanism.

## ✨ Key Features

- **High Performance:** Built with Rust, featuring `revm` for EVM execution and a parallel transaction scheduler.
- **EVM Compatibility:** Full support for Ethereum Virtual Machine, allowing seamless migration of smart contracts.
- **Modular Architecture:** Pluggable consensus engines (PoA, PoS, etc.), customizable storage backends, and modular components.
- **Developer-Friendly:** Comprehensive CLI, Rust and JavaScript SDKs, and detailed documentation.
- **Asynchronous & Parallel:** Designed from the ground up to maximize performance on multi-core systems.

## 🚀 Quick Start

### 1. Prerequisites

- [Rust & Cargo](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/) (for devnet)

### 2. Build the Node

```bash
git clone https://github.com/velora-chain/velora-chain.git
cd velora-chain
cargo build --release
```

### 3. Run a Local Devnet

The easiest way to get a local test network running is to use the provided `devnet.sh` script. This will start a 4-node network with pre-configured validators and accounts.

```bash
# From the project root
bash ./scripts/devnet.sh
```

This will:
- Clean up any previous devnet data.
- Generate genesis and node key files.
- Start 4 `velora` nodes connected to each other.
- Expose JSON-RPC endpoints on ports `8545` through `8548`.

### 4. Interact with the Network

You can now use standard Ethereum tools like `foundry` or `hardhat` to interact with your local devnet.

**RPC Endpoint:** `http://localhost:8545`
**Chain ID:** `1337`

## 📚 Documentation

For more in-depth information, please refer to our official documentation:

- [Architecture Overview](./docs/architecture.md)
- [Developer Guide](./docs/dev_guide.md)
- [How to Run a Node](./docs/how_to_run.md)

## 🤝 Contributing

We welcome contributions from the community! Please read our [Contributing Guide](./CONTRIBUTING.md) to get started.

## 📄 License

Velora is licensed under the [MIT License](./LICENSE) or [Apache-2.0 License](./LICENSE-APACHE).
