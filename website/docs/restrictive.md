# Restrictive mode

The --restrictive-mode argument enables a restrictive mode for devnet, allowing you to specify methods that are forbidden during execution. This option ensures that certain operations are restricted, enhancing control over the devnet behavior. When user sends a request to one of the restricted methods it will return either JSON-RPC error with code -32604 or HTTP status 403.

## Usage

### With default methods

```
$ starknet-devnet --restrictive-mode
```

### With a list of methods

```
$ starknet-devnet --restrictive-mode devnet_dump devnet_config
```

## Default restricted methods

When no methods are specified, the following default methods will be restricted and their HTTP endpoints counterparts (if any):

- devnet_mint
- devnet_restart
- devnet_createBlock
- devnet_abortBlocks
- devnet_impersonateAccount
- devnet_autoImpersonate
- devnet_getPredeployedAccounts
