# Restrictive mode

The --restrictive-mode argument enables a restrictive mode for the devnet, allowing you to specify methods that should be forbidden during execution. This option ensures that certain operations are restricted, enhancing control over the devnet behavior. When user sends a request to one of the restricted methods it will return either JSON-RPC error with code -32604 or HTTP status 403.

## Usage

Devnet will restrict default methods:

```
$ starknet-devnet --restrictive-mode
```

Devnet will restrict methods passed to the argument

```
$ starknet-devnet --restrictive-mode devnet_dump devnet_config
```

## Default Restricted Methods

When no methods are specified, the following default methods will be restricted:

- devnet_mint
- devnet_restart
- devnet_createBlock
- devnet_abortBlocks
- devnet_impersonateAccount
- devnet_autoImpersonate
- devnet_getPredeployedAccounts
