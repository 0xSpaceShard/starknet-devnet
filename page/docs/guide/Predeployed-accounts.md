---
sidebar_position: 13
---

# Predeployed accounts

Devnet predeploys `--accounts` with some `--initial-balance`. The accounts get charged for transactions according to the `--gas-price`. A `--seed` can be used to regenerate the same set of accounts. Read more about it in the [Run section](#run).

To get the code of the account (currently OpenZeppelin v0.3.1), use one of the following:

- `GET /get_code?contractAddress=<ACCOUNT_ADDRESS>`
- [**Starknet CLI**](https://www.cairo-lang.org/docs/hello_starknet/cli.html#get-code): `starknet get_code --contract_address <ACCOUNT_ADDRESS> --feeder_gateway_url <DEVNET_URL>`
- [**OpenZeppelin's cairo-contract repository**](https://github.com/OpenZeppelin/cairo-contracts/tree/v0.3.1)

You can use the accounts in e.g. [**starknet-hardhat-plugin**](https://github.com/Shard-Labs/starknet-hardhat-plugin) via:

```typescript
const account = await starknet.getAccountFromAddress(
  ADDRESS,
  PRIVATE_KEY,
  "OpenZeppelin"
);
```
## Fetch predeployed accounts

```
GET /predeployed_accounts
```

Response:

```
[
  {
    "initial_balance": 1e+21,
    "address": "0x7c3e2...",
    "private_key": "0x6160...",
    "public_key": "0x6a5540..."
  },
  ...
]
```

## Fetch account balance

```
GET /account_balance?address=<HEX_ADDRESS>
```

Response:

```
{
  "amount": 123...456,
  "unit": "wei"
}
```