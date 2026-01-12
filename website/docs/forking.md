# Forking

To interact with contracts deployed on mainnet or testnet, you can use forking. Simulate the origin and experiment with it locally, making no changes to the origin itself.

```
$ starknet-devnet --fork-network <URL> [--fork-block <BLOCK_NUMBER>]
```

The value passed to `--fork-network` should be the URL to a Starknet JSON-RPC API provider. Specifying a `--fork-block` is optional; it defaults to the `"latest"` block at the time of Devnet's start-up. All calls will first try Devnet's state and then fall back to the forking block.

:::note How it works

When you send a request to a forked Devnet instance, it first queries Devnet's local state, then tries the forking origin. Forking is not a step simply performed on Devnet startup, but happens continuously while the Devnet instance is alive.

:::

## Upstream caching

By default, Devnet does not cache responses from the fork upstream network. However, you can enable caching to improve performance and reduce the number of requests sent to the origin by using the `--fork-upstream-caching` flag:

```
$ starknet-devnet --fork-network <URL> --fork-upstream-caching
```

:::tip Performance improvement

Enabling upstream caching can significantly improve performance by reducing redundant requests to the fork origin.

:::

## Forking and ACCEPTED_ON_L1

Assume you have run Devnet as a fork from an origin at a block that is not yet `ACCEPTED_ON_L1`, but only `ACCEPTED_ON_L2`. If in your state queries you specify `block_id: "l1_accepted"`, and there are no local blocks marked as `ACCEPTED_ON_L1`, Devnet will assume the forking block has become `ACCEPTED_ON_L1`.

Read more about marking blocks as `ACCEPTED_ON_L1` on Devnet: [link](./blocks#accepting-blocks-on-l1).

## Account impersonation

[Here](./account-impersonation) you can read more about acting as an account deployed on the origin.

## Deploying an undeclared account

[Here](./predeployed#deploying-an-undeclared-account) you can read about deploying an account not declared on Devnet.

## Checking forking status

To see if your Devnet instance is using forking or not, [fetch the current configuration](./api#config-api), and check the `url` property of its `fork_config` property. If Devnet is forked, this property contains the string of the origin URL specified on startup.
