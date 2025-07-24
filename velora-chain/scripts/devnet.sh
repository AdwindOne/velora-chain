#!/bin/bash

set -e
set -u

# --- Configuration ---
NODES=${NODES:-4}
BASE_P2P_PORT=${BASE_P2P_PORT:-30303}
BASE_RPC_PORT=${BASE_RPC_PORT:-8545}
DATA_DIR=${DATA_DIR:-".velora-devnet"}
BINARY_NAME="velora"
BINARY_PATH="./target/release/$BINARY_NAME"
CHAIN_ID=${CHAIN_ID:-1337}
IP_ADDR="127.0.0.1"

# --- Cleanup ---
echo "Cleaning up old devnet data..."
rm -rf $DATA_DIR
pkill -f $BINARY_NAME || true
sleep 1

# --- Build ---
echo "Building Velora binary..."
cargo build --release --bin $BINARY_NAME

# --- Genesis ---
GENESIS_PATH="$DATA_DIR/genesis.json"
mkdir -p $(dirname $GENESIS_PATH)
cp ./configs/devnet/genesis.json $GENESIS_PATH

# --- Start Nodes ---
echo "Starting $NODES Velora nodes..."
for i in $(seq 1 $NODES); do
  NODE_DIR="$DATA_DIR/node$i"
  P2P_PORT=$((BASE_P2P_PORT + i - 1))
  RPC_PORT=$((BASE_RPC_PORT + i - 1))

  echo "Starting Node $i: P2P Port=$P2P_PORT, RPC Port=$((8545 + i -1))"

  # Initialize node-specific data
  $BINARY_PATH init --datadir $NODE_DIR --genesis $GENESIS_PATH > /dev/null 2>&1

  $BINARY_PATH run \
    --datadir $NODE_DIR \
    --p2p-port $P2P_PORT \
    --rpc-addr "127.0.0.1:$((8545 + i - 1))" \
    > "$NODE_DIR/node.log" 2>&1 &
done

echo "✅ Velora Devnet started with $NODES nodes."
echo "Logs are in $DATA_DIR/node*/node.log"
echo "RPC endpoints are available at http://$IP_ADDR:8545 to http://$IP_ADDR:$((BASE_RPC_PORT + NODES - 1))"
