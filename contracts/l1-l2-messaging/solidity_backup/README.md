## Solidity Contracts

Contracts in this folder are Solidity contracts related to L1 node.
Some contracts are used by Devnet only, and others are examples to help you testing Devnet features.

## Work with Forge

First, install forge following the [official book](https://book.getfoundry.sh/getting-started/installation).

### Build

To build the contracts, enter the solidity directory and run:
```bash
forge build
```
This will generates the artifacts of the contracts into the `out` folder.

### Testing

### Scripting

To easily interact with a Ethereum node, you can script using solidity.
Some examples can be found in the `script` folder.
To run them, you can do the following:
```bash
# Run anvil / hardhat in a separate terminal

# You can copy .env.anvil to .env for quick setup.
cp .env.anvil .env

# You can do source .env where you can define your ETH_RPC_URL and other variables.
source .env

# Then run:
forge script script/L1L2.s.sol:Deploy --broadcast --rpc-url $ETH_RPC_URL --silent

# You'll see the report like this to check the address of your contract.
✅  [Success]Hash: 0x79aaf3bc7be7b6dd90fd897f4efa2f25c98b94d0b6a5d20df6b808b8a2f3b7df
Contract Address: 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
Block: 3
Paid: 0.001173141032566412 ETH (311164 gas * 3.770169533 gwei)
```

## MockStarknetMessaging

The `MockStarknetMessaging` contract is the contract that is automatically
deployed by Devnet on the L1 node. This contracts is mocking the
[starknet core contract](https://etherscan.io/address/0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4) on
Ethereum by handling the messages, without requiring a proof to be verified.

The Devnet works with [ethers](https://docs.rs/ethers/latest/ethers/index.html) rust crate to
interact with ethereum. Using the `abigen` macro, the `MockStarknetMessaging` artifacts is required.
And this artifact **must** contains both the `ABI` and the `bytecode` as Devnet can also deploy it.

You can use `forge build` to generate the artifacts, and then copying the artifacts for `MockStarknetMessaging`
into `artifacts` folder. Or you can use `generate_artifacts.sh` script.
