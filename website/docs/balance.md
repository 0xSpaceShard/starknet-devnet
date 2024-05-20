# Account balance

Other than using prefunded predeployed accounts, you can also add funds to an account that you deployed yourself.

Separate tokens use separate ERC20 contracts for minting and charging fees. These are the token contracts predeployed by Devnet and the addresses where they are located:

- ETH: `0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7`
- STRK: `0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d`

## Mint token - Local faucet

By sending a `POST` request to `/mint` for a token, you initiate a transaction on that token's ERC20 contract. The response contains the hash of this transaction, as well as the new balance after minting. The token is specified by providing the unit, and defaults to `WEI`.

The value of `amount` is in WEI and needs to be an integer (or a float whose fractional part is 0, e.g. `1000.0` or `1e21`)

```
POST /mint
{
    "address": "0x6e3205f...",
    "amount": 500000,
    "unit": "WEI" | "FRI"
}
```

Response:

```
{
    "new_balance": 500000,
    "unit": "WEI" | "FRI",
    "tx_hash": "0xa24f23..."
}
```

## Check balance

Check the balance of an address by sending a `GET` request to `/account_balance`. The address should be a 0x-prefixed hex string; `unit` defaults to `WEI` and `block_tag` to `latest`.

```
GET /account_balance?address=<ADDRESS>[&unit=<FRI|WEI>][&block_tag=<latest|pending>]
```
