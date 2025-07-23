# Velora Architecture

Velora is designed with a modular and layered architecture to maximize performance, flexibility, and maintainability. This document provides a high-level overview of the key components and their interactions.

## 1. Core Principles

- **Modularity:** Each core function (consensus, execution, networking) is a separate crate, allowing for independent development, testing, and even replacement.
- **Performance:** Leveraging Rust's performance and safety, along with parallel execution strategies, to achieve high throughput.
- **EVM Compatibility:** Full compatibility with the Ethereum Virtual Machine (EVM) is a primary goal, ensuring that existing smart contracts and tools work seamlessly.

## 2. Layers

The Velora node is composed of several distinct layers that work together to form a cohesive system.

```
+---------------------+
|         RPC         |
+---------------------+
|      TxPool         |
+---------------------+
|      Executor       |
+---------------------+
|     Consensus       |
+---------------------+
|      Networking     |
+---------------------+
|        Storage      |
+---------------------+
```

### 2.1. RPC Layer (`rpc` crate)

- **Purpose:** Exposes the functionality of the node to the outside world via a JSON-RPC API.
- **Implementation:** Built using the `jsonrpsee` library, supporting both HTTP and WebSocket connections.
- **Features:**
    - Standard Ethereum JSON-RPC methods (`eth_*`, `web3_*`, `net_*`).
    - Custom `velora_*` methods for node-specific information and control.
    - Tracing and debugging endpoints.

### 2.2. Transaction Pool (`txpool` crate)

- **Purpose:** Manages incoming transactions before they are included in a block.
- **Implementation:** A concurrent, in-memory pool that prioritizes and validates transactions.
- **Features:**
    - Nonce-based transaction ordering.
    - Gas price-based prioritization.
    - Conflict detection to prevent double-spending.
    - Readiness for parallel execution by identifying non-conflicting transactions.

### 2.3. Executor Layer (`executor` crate)

- **Purpose:** Executes the state transitions defined by transactions.
- **Implementation:**
    - **EVM Engine:** Integrates `revm`, a fast and efficient EVM implementation in Rust.
    - **State Management:** Manages the world state using a Merkle-Patricia Trie.
    - **Database Backend:** Uses RocksDB or LMDB for persistent storage.
- **Features:**
    - **Parallel Execution:** A task scheduler (like `pevm` or a custom implementation) will be used to execute non-conflicting transactions in parallel.
    - State snapshotting and pruning for efficient operation.

### 2.4. Consensus Layer (`consensus` crate)

- **Purpose:** Enables nodes to agree on the state of the network.
- **Implementation:** A pluggable consensus engine system.
- **Features:**
    - **Initial Engine:** A simple Proof-of-Authority (PoA) engine for devnets and testnets.
    - **Future Engines:** Designed to easily accommodate other consensus mechanisms like Proof-of-Stake (PoS) or custom BFT variants.

### 2.5. Networking Layer (`network` crate)

- **Purpose:** Handles all peer-to-peer (P2P) communication between nodes.
- **Implementation:** Built on top of the `libp2p` framework.
- **Features:**
    - **Gossip Protocol:** For broadcasting new blocks and transactions.
    - **Peer Discovery:** For finding and connecting to other nodes in the network.
    - **Sync Protocol:** For new nodes to catch up to the latest state of the chain.

### 2.6. Storage Layer

- **Purpose:** Provides a persistent key-value store for all blockchain data.
- **Implementation:** Abstracted to support multiple database backends.
- **Features:**
    - **Default Backend:** RocksDB for its high performance and battle-tested reliability.
    - **Alternative Backend:** LMDB for its transactional guarantees and read performance.
    - Stores everything: block headers, block bodies, transaction receipts, and the state trie.

## 3. Parallel EVM Execution

A key feature of Velora is its ability to execute EVM transactions in parallel. This is achieved through the following process:

1.  **Transaction Identification:** The `TxPool` identifies transactions that do not have conflicting state access (e.g., they don't modify the same storage slots).
2.  **Task Scheduling:** A scheduler assigns these non-conflicting transactions to different worker threads.
3.  **Parallel Execution:** Each worker thread has its own instance of `revm` and executes its assigned transaction in parallel with other threads.
4.  **State Aggregation:** After execution, the state changes from all threads are aggregated and committed to the main state trie.

This approach significantly increases the transaction throughput of the node compared to the sequential execution model used in traditional EVM chains.
