# Velora Devnet Guide

The Velora devnet is a local, multi-node test network designed for developers to test their smart contracts and applications in a realistic environment. This guide explains how to use it.

## 1. Starting the Devnet

The easiest way to start the devnet is to use the provided script:

```bash
# From the root of the project
bash ./scripts/devnet.sh
```

This will:
- Start 4 `velora` nodes on your local machine.
- Create a `.velora-devnet` directory to store all node data.
- Expose JSON-RPC endpoints on ports `8545` through `8548`.

## 2. Devnet Configuration

- **Chain ID:** `1337`
- **Network ID:** `1337`
- **Consensus:** Proof-of-Authority (PoA)
- **RPC Endpoints:**
  - Node 1: `http://127.0.0.1:8545`
  - Node 2: `http://127.0.0.1:8546`
  - Node 3: `http://127.0.0.1:8547`
  - Node 4: `http://127.0.0.1:8548`
- **Pre-funded Account:**
  - **Address:** `0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b`
  - **Private Key (for testing only):** `0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`
  - **Balance:** `1,000,000` VLR (or `10^24` wei)

**Warning:** The private key provided above is for testing purposes only. Do not use it on any mainnet.

## 3. Connecting to the Devnet

You can connect to the devnet using any standard Ethereum tool, such as Foundry, Hardhat, or MetaMask.

### 3.1. Foundry

To use Foundry with the devnet, you can set the `ETH_RPC_URL` environment variable:

```bash
export ETH_RPC_URL=http://127.0.0.1:8545
```

Now, you can use `forge` and `cast` to interact with the network.

**Example: Get the chain ID**
```bash
cast chain-id
# Expected output: 1337
```

**Example: Get the balance of the pre-funded account**
```bash
cast balance 0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b
# Expected output: 1000000000000000000000000
```

### 3.2. Hardhat

To use Hardhat, you need to configure your `hardhat.config.js` file to point to the devnet.

```javascript
module.exports = {
  solidity: "0.8.20",
  networks: {
    velora_devnet: {
      url: "http://127.0.0.1:8545",
      chainId: 1337,
      accounts: ['0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80']
    }
  }
};
```

Now, you can run your Hardhat tasks on the devnet:
```bash
npx hardhat run scripts/deploy.js --network velora_devnet
```

### 3.3. MetaMask

To connect MetaMask to the devnet, you can add a new custom network with the following settings:

- **Network Name:** Velora Devnet
- **New RPC URL:** `http://127.0.0.1:8545`
- **Chain ID:** `1337`
- **Currency Symbol:** VLR

You can then import the pre-funded account using its private key to have funds available for testing.

## 4. Deploying Contracts

You can deploy smart contracts to the devnet using your preferred development tool.

### Example: Deploying with Foundry

1.  Create a new Foundry project:
    ```bash
    forge init my-contract
    cd my-contract
    ```
2.  Write your contract (e.g., `src/Counter.sol`).
3.  Deploy the contract to the devnet:
    ```bash
    forge create src/Counter.sol:Counter \
      --rpc-url http://127.0.0.1:8545 \
      --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
    ```

This will deploy the `Counter` contract to the Velora devnet and print its address.

## 5. Viewing Logs

The logs for each node are stored in the `.velora-devnet` directory. You can view the logs for a specific node like this:

```bash
tail -f .velora-devnet/node1/node.log
```

This is useful for debugging issues with the node or your smart contracts.
