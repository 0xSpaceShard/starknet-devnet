# Predeployed contracts

Devnet predeploys a [UDC](https://docs.openzeppelin.com/contracts-cairo/0.6.1/udc), an [ERC20 (fee token)](https://docs.openzeppelin.com/contracts-cairo/0.8.1/erc20) contract and a set of predeployed funded accounts.

The set of accounts can be controlled via [CLI options](./running/cli): `--accounts <NUMBER_OF>`, `--initial-balance <WEI>`, `--seed <VALUE>`.

## Account class selection

Choose between predeploying Cairo 0 (OpenZeppelin 0.5.1) or Cairo 1 (default; OpenZeppelin 0.8.1) accounts by using:

```
--account-class [cairo0 | cairo1]
```

Alternatively, provide a path to the [Sierra artifact](https://github.com/starkware-libs/cairo#compiling-and-running-cairo-files) of your custom account using:

```
--account-class-custom <SIERRA_PATH>
```

## Deploying an undeclared account

If you want to deploy an instance of an account contract class not predeclared on Devnet, you can use [forking](./forking). Just fork an origin network which has the needed class already declared, e.g. the Sepolia testnet. Why? Because new versions of wallets like ArgentX and Braavos tend to be declared on testnet/mainnet soon after release.

## How to get predeployment info?

The information on predeployed contracts is logged on Devnet startup. Predeployed accounts can be retrieved in JSON format by sending a `GET` request to `/predeployed_accounts`. With the additional query parameter `with_balance=true`, WEI and FRI balances at the pending state will be provided:

```
GET /predeployed_accounts?[with_balance=true]
```

Alternatively, send a JSON-RPC request:

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_getPredeployedAccounts"
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_getPredeployedAccounts",
    "params": {
        // optional; defaults to false
        "with_balance": true | false
    }
}
```
