#!/bin/bash

set -e
set -u

# --- Configuration ---
NODES=${NODES:-4}
BASE_P2P_PORT=${BASE_P2P_PORT:-30303}
BASE_RPC_PORT=${BASE_RPC_PORT:-8545}
DATA_DIR=${DATA_DIR:-".velora-devnet"}
BINARY_PATH=${BINARY_PATH:-"./target/release/velora"}
CHAIN_ID=${CHAIN_ID:-1337}
IP_ADDR="127.0.0.1"

# --- Cleanup ---
echo "Cleaning up old devnet data..."
rm -rf $DATA_DIR
pkill -f $BINARY_PATH || true
sleep 1

# --- Build ---
if [ ! -f "$BINARY_PATH" ]; then
    echo "Velora binary not found. Building..."
    cargo build --release --bin velora
fi

# --- Generate Node Keys & Genesis ---
echo "Generating node keys and genesis file..."
mkdir -p $DATA_DIR/genesis
declare -a BOOTNODE_URLS

for i in $(seq 1 $NODES); do
    NODE_DIR="$DATA_DIR/node$i"
    mkdir -p $NODE_DIR/keystore
    # In a real scenario, we'd use a proper key generation tool
    NODE_KEY=$(echo "node$i" | sha256sum | head -c 64)
    echo $NODE_KEY > $NODE_DIR/node.key

    # This is a placeholder for a real enode URL generation
    if [ $i -eq 1 ]; then
        BOOTNODE_URL="/ip4/$IP_ADDR/tcp/$BASE_P2P_PORT/p2p/$(echo $NODE_KEY | head -c 32)" # Simplified for demo
    fi
done

# Create a simple PoA genesis file
cat <<EOF > $DATA_DIR/genesis/genesis.json
{
  "genesis": {
    "nonce": "0x0000000000000042",
    "timestamp": "0x0",
    "extraData": "0x",
    "gasLimit": "0x1000000",
    "difficulty": "0x1",
    "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "coinbase": "0x0000000000000000000000000000000000000000",
    "alloc": {},
    "number": "0x0",
    "gasUsed": "0x0",
    "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
  },
  "params": {
    "chainId": $CHAIN_ID,
    "networkID": $CHAIN_ID
  },
  "accounts": {
    "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b": {
      "balance": "1000000000000000000000"
    }
  }
}
EOF

# --- Start Nodes ---
echo "Starting $NODES Velora nodes..."
for i in $(seq 1 $NODES); do
  NODE_DIR="$DATA_DIR/node$i"
  P2P_PORT=$((BASE_P2P_PORT + i - 1))
  RPC_PORT=$((BASE_RPC_PORT + i - 1))
  NODE_KEY_PATH="$NODE_DIR/node.key"

  echo "Starting Node $i: P2P Port=$P2P_PORT, RPC Port=$RPC_PORT"

  $BINARY_PATH \
    --datadir $NODE_DIR \
    --listen-addr "/ip4/$IP_ADDR/tcp/$P2P_PORT" \
    --rpc-port $RPC_PORT \
    --nodekey-path $NODE_KEY_PATH \
    --bootnodes "$BOOTNODE_URLS" \
    --chain $DATA_DIR/genesis/genesis.json \
    --dev \
    > "$NODE_DIR/node.log" 2>&1 &
done

echo "✅ Velora Devnet started with $NODES nodes."
echo "Logs are in $DATA_DIR/node*/node.log"
echo "RPC endpoints are available at http://$IP_ADDR:8545 to http://$IP_ADDR:$((BASE_RPC_PORT + NODES - 1))"
