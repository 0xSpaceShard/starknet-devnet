# Forking

To interact with contracts deployed on mainnet or testnet, you can use forking. Simulate the origin and experiment with it locally, making no changes to the origin itself.

```
$ starknet-devnet --fork-network <URL> [--fork-block <BLOCK_NUMBER>]
```

The value passed to `--fork-network` should be the URL to a Starknet JSON-RPC API provider. Specifying a `--fork-block` is optional; it defaults to the `"latest"` block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block.

## Account impersonation

[Here](./account-impersonation) you can read more about acting as an account deployed on the origin.

## Deploying an undeclared account

[Here](./predeployed#deploying-an-undeclared-account) you can read about deploying an account not declared on Devnet.
