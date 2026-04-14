# Lite mode

To run Devnet in a minimal lite mode, provide the flag:

```
$ starknet-devnet --lite-mode
```

Steps skipped in lite mode:

- **Calculating block hash**: Block hash is simplified and derived from the block number instead of computing the full cryptographic hash
- **Calculating block commitments**: Skips computation of transaction commitment, event commitment, state diff commitment, and receipt commitment

In lite mode, blocks are still created and transactions are executed normally, but the computationally expensive cryptographic operations for generating block hashes and commitments are bypassed. This improves performance for development and testing scenarios where these values are not required.

This is useful if your use-case doesn't need the functionalities above, such as when:

- Testing transaction execution logic
- Running integration tests that don't verify block hashes or commitments
- Rapid prototyping and development iteration

The extent of what is affected by lite mode may be expanded in the future.
