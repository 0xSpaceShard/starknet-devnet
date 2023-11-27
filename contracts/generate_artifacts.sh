#!/bin/bash

# Bash script to generate the artifacts required by Devnet to compile.
mkdir -p artifacts

# L1-L2 messaging:
## SOLIDITY
forge build --root ./l1-l2-messaging/solidity
cp ./l1-l2-messaging/solidity/out/MockStarknetMessaging.sol/MockStarknetMessaging.json ./artifacts/

## CAIRO
scarb --manifest-path ./l1-l2-messaging/cairo/Scarb.toml build
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2.contract_class.json ./artifacts/
