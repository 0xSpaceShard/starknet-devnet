---
sidebar_position: 10
---

# Restart

Devnet can be restarted by making a `POST /restart` request. All of the deployed contracts, blocks and storage updates will be restarted to the empty state. If you're using [**the Hardhat plugin**](https://github.com/0xSpaceShard/starknet-hardhat-plugin#restart), run `await starknet.devnet.restart()`.
