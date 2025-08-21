# Predeployed contracts

Devnet predeploys:

- two [UDCs](https://docs.openzeppelin.com/contracts-cairo/udc)
  - on the same addresses as Starknet mainnet/testnet
  - a legacy instance serving only the `deployContract` entrypoint
  - a UDC 2 instance serving both `deployContract` and `deploy_contract` entrypoints
- two [ERC20 (fee token)](https://docs.openzeppelin.com/contracts-cairo/erc20) contracts
  - on the same addresses as Starknet mainnet/testnet
  - one for STRK
  - one for ETH
- a set of predeployed funded accounts
  - not a feature of Starknet mainnet/testnet.

The set of accounts can be controlled via [CLI options](./running/cli): `--accounts <NUMBER_OF>`, `--initial-balance <WEI>`, `--seed <VALUE>`.

## Predeployed account preservation

:::note

Once you shut down your Devnet, the predeployed account you used ceases to exist. This may be a problem with tools such as `starkli` which hardcode your account details in a local file. One option then is to delete your account entry from `starkli`'s account file. Another option is to spawn the same account on next Devnet startup. To do this, you can use:

- the `--seed <VALUE>` CLI option which always predeploys the same set of accounts if using the same `<VALUE>` (the seed is logged on startup)
- the [dump and load feature](./dump-load-restart)

:::

## Account class selection

Choose between predeploying Cairo 0 (OpenZeppelin 0.5.1) or Cairo 1 (default; OpenZeppelin 1.0.0) accounts by using:

```
--account-class [cairo0 | cairo1]
```

Alternatively, provide a path to the [Sierra artifact](https://github.com/starkware-libs/cairo#compiling-and-running-cairo-files) of your custom account using:

```
--account-class-custom <SIERRA_PATH>
```

## Predeclared account classes

Both Cairo 0 and Cairo 1 versions of OpenZeppelin account are always predeclared, regardless of the chosen predeployment variant. If you specify `--predeclare-argent` on startup, the latest regular and multisig Argent account variants will also be predeclared. All predeclared classes can be used in DEPLOY_ACCOUNT transactions.

## Deploying an undeclared account

If you want to deploy an instance of an account contract class not predeclared on Devnet, you can use [forking](./forking). Just fork an origin network which has the needed class already declared, e.g. the Sepolia testnet. Why? Because new versions of wallets like ArgentX and Braavos tend to be declared on testnet/mainnet soon after release.

## How to get predeployment info?

### Logged on startup

The startup log includes:

- the hash values of predeclared classes
- the addresses of predeployed contracts
- the keys and initial balance of predeployed account contracts

### API

Account class info can be found in the response to [config request](api#config-api).

Predeployed account details can be retrieved in JSON format by sending a `GET` request to `/predeployed_accounts` or via JSON-RPC. With the additional query parameter `with_balance` set to `true`, ETH and STRK balances at the pre-confirmed state will be provided, in WEI and FRI, respectively:

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
