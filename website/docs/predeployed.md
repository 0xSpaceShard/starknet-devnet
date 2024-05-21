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

## How to get predeployment info?

The predeployment information is logged on Devnet startup. Predeployed accounts can be retrieved in JSON format by sending a `GET` request to `/predeployed_accounts` of your Devnet.
