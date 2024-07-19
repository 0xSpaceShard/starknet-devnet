# Restrictive mode

The --restrictive-mode argument enables a restrictive mode for Devnet, allowing you to specify methods that are forbidden during execution. This option ensures that certain operations are restricted, enhancing control over Devnet's behavior. When a user sends a request to one of the restricted methods, Devnet will return either a JSON-RPC error with code -32604 or, if the method was targeted directly via the HTTP endpoint, a response with status 403.

## Default restricted methods

When no methods are specified, the following default methods will be restricted and their HTTP endpoints counterparts (if any):

- devnet_mint
- devnet_restart
- devnet_createBlock
- devnet_abortBlocks
- devnet_impersonateAccount
- devnet_autoImpersonate
- devnet_getPredeployedAccounts

## Usage

### With default methods

```
$ starknet-devnet --restrictive-mode
```

### With a list of methods

```
$ starknet-devnet --restrictive-mode devnet_dump devnet_config
```
