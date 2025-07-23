# How to Run a Velora Node

This guide provides step-by-step instructions for building, configuring, and running a `velora` node.

## 1. Prerequisites

Before you begin, ensure you have the following installed:

- **Rust:** [Install Rust and Cargo](https://www.rust-lang.org/tools/install)
- **Git:** [Install Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
- **C++ Compiler:** A C++ compiler is required for RocksDB.
  - On Debian/Ubuntu: `sudo apt-get install build-essential`
  - On macOS: `xcode-select --install`

## 2. Building the Node

First, clone the repository and build the `velora` binary.

```bash
# Clone the repository
git clone https://github.com/velora-chain/velora-chain.git
cd velora-chain

# Build the binary in release mode
cargo build --release
```

The compiled binary will be located at `target/release/velora`.

## 3. Running a Local Devnet (Recommended)

The quickest way to get a network up and running is to use the provided devnet script. This will start a multi-node network on your local machine.

```bash
# From the root of the project
bash ./scripts/devnet.sh
```

This script will:
- Build the `velora` binary if it doesn't exist.
- Create a `.velora-devnet` directory to store all node data.
- Generate node keys and a `genesis.json` file.
- Start 4 nodes, each with its own data directory, P2P port, and RPC port.
- Connect the nodes to each other.

You can interact with the devnet via the following RPC endpoints:
- Node 1: `http://127.0.0.1:8545`
- Node 2: `http://127.0.0.1:8546`
- Node 3: `http://127.0.0.1:8547`
- Node 4: `http://127.0.0.1:8548`

To stop the devnet, you can run:
```bash
pkill -f target/release/velora
```

## 4. Running a Single Node Manually

If you want to run a single node, you can do so with the `run` subcommand.

### 4.1. Initialize the Node

First, you need to initialize the node's data directory and create a genesis file.

```bash
# Create a data directory
mkdir my-node

# Create a genesis file (e.g., my-node/genesis.json)
# You can copy the one from `configs/devnet/genesis.json` as a starting point.
cp configs/devnet/genesis.json my-node/genesis.json

# Generate a node key
# (This is a placeholder, a real tool would be needed for this)
echo "my_node_key" | sha256sum | head -c 64 > my-node/node.key
```

### 4.2. Start the Node

Now, you can start the node using the `run` command, pointing it to your data directory and genesis file.

```bash
./target/release/velora run \
  --datadir ./my-node \
  --chain ./my-node/genesis.json \
  --nodekey-path ./my-node/node.key \
  --listen-addr "/ip4/127.0.0.1/tcp/30303" \
  --rpc-port 8545 \
  --dev
```

### 4.3. Command-Line Flags

Here are some of the most common flags for the `run` command:

- `--datadir <PATH>`: The directory to store the node's data.
- `--chain <PATH>`: The path to the genesis file.
- `--nodekey-path <PATH>`: The path to the file containing the node's private key.
- `--listen-addr <MULTIADDR>`: The P2P address to listen on (e.g., `/ip4/0.0.0.0/tcp/30303`).
- `--rpc-port <PORT>`: The port to run the JSON-RPC server on.
- `--bootnodes <MULTIADDR>`: A comma-separated list of bootnodes to connect to.
- `--dev`: Run in development mode (e.g., with instant sealing).

For a full list of available flags, run:
```bash
./target/release/velora run --help
```

## 5. Connecting to a Network

To connect your node to an existing network, you'll need two things:

1.  **The network's `genesis.json` file.**
2.  **The multiaddresses of one or more bootnodes.**

Once you have these, you can start your node with the `--bootnodes` flag:

```bash
./target/release/velora run \
  --datadir ./my-node \
  --chain /path/to/genesis.json \
  --nodekey-path ./my-node/node.key \
  --listen-addr "/ip4/0.0.0.0/tcp/30303" \
  --bootnodes "/ip4/10.0.0.1/tcp/30303/p2p/Qm..._bootnode_1,/ip4/10.0.0.2/tcp/30303/p2p/Qm..._bootnode_2"
```

Your node will then connect to the bootnodes and start syncing with the rest of the network.
