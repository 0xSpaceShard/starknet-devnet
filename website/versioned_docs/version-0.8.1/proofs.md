# Transaction proofs and proof modes

This page describes the Devnet-specific transaction proof flow introduced in Starknet v0.14.2 and the `--proof-mode` configuration.

:::caution Not about storage proofs

`starknet_getStorageProof` (Merkle-Patricia storage proofs) is **not supported** in Devnet. This page covers `starknet_proveTransaction`, a separate Devnet extension for proving/validating `INVOKE v3` transaction payloads.

:::

## Proof modes

Proof behavior is controlled by `--proof-mode` (or env var `PROOF_MODE`).

### Mode summary

| Mode   | CLI value          | What `starknet_proveTransaction` does            | How `starknet_addInvokeTransaction` treats `proof` and `proof_facts`                                                     |
| ------ | ------------------ | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Full   | `full`             | Not implemented yet                              | Rejects with unsupported action                                                                                          |
| Devnet | `devnet` (default) | Returns a deterministic mock proof + proof facts | If both fields are present, verifies them; if one is missing or verification fails, rejects; if both are absent, accepts |
| None   | `none`             | Disabled / unsupported                           | Ignores incoming `proof` for invoke txs                                                                                  |

### Why this exists

`devnet` mode is a practical mode for local testing:

- You can request a proof for a specific `INVOKE v3` payload.
- You can later attach that proof to a transaction and exercise proof-aware flows.
- Verification is deterministic and lightweight, intended for development workflows rather than real on-chain proving.

## Configuration

### CLI

```bash
starknet-devnet --proof-mode devnet
```

```bash
starknet-devnet --proof-mode none
```

```bash
starknet-devnet --proof-mode full
```

### Environment variable

```bash
PROOF_MODE=devnet starknet-devnet
```

### Docker

```bash
docker run --rm -p 5050:5050 \
  -e PROOF_MODE=devnet \
  shardlabs/starknet-devnet-rs
```

## RPC: `starknet_proveTransaction`

### Request shape

`starknet_proveTransaction` accepts:

- `block_id`
- `transaction` (a broadcasted `INVOKE v3` transaction payload)

Example:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "starknet_proveTransaction",
  "params": {
    "block_id": "latest",
    "transaction": {
      "type": "INVOKE",
      "version": "0x3",
      "sender_address": "0x1234",
      "calldata": ["0x1", "0x2"],
      "signature": [],
      "nonce": "0x0",
      "resource_bounds": {
        "l1_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
        "l1_data_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
        "l2_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" }
      },
      "tip": "0x0",
      "paymaster_data": [],
      "account_deployment_data": [],
      "nonce_data_availability_mode": "L1",
      "fee_data_availability_mode": "L1"
    }
  }
}
```

### Response shape

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "proof": "<base64-encoded-proof>",
    "proof_facts": [
      "0x...",
      "0x...",
      "0x...",
      "0x...",
      "0x...",
      "0x...",
      "0x...",
      "0x...",
      "0x..."
    ],
    "l2_to_l1_messages": [
      {
        "order": 0,
        "from_address": "0x...",
        "to_address": "0x...",
        "payload": ["0x...", "0x..."]
      }
    ]
  }
}
```

`proof_facts` length is expected to be 9 in devnet mode (includes a `messages_hash` element derived from L2→L1 messages).

`l2_to_l1_messages` contains L2→L1 messages extracted by simulating the transaction. If the simulation fails (e.g. execution reverts), `starknet_proveTransaction` returns an error instead of a proof.

## Mode-specific behavior details

### `devnet` mode (default)

- `starknet_proveTransaction` is enabled.
- Invoke handling rules:
  - both `proof` + `proof_facts` present and valid → accepted
  - both present but invalid → rejected
  - only one present → rejected
  - both absent → accepted

### `none` mode

- Proof field on invoke transactions is ignored; `proof_facts` are checked.

### `full` mode

- Full proving/verification is not implemented yet.
- Endpoints and transactions requiring full verification return unsupported-action errors.
