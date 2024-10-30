#!/bin/bash
set -eu

# Bash script to generate the artifacts required by Devnet to compile.
mkdir -p l1-l2-artifacts
mkdir -p ../crates/starknet-devnet-core/contracts/l1-l2-artifacts

# L1-L2 messaging:
## SOLIDITY
forge build --root ./l1-l2-messaging/solidity
cp ./l1-l2-messaging/solidity/out/L1L2.sol/L1L2Example.json ./l1-l2-artifacts/
cp ./l1-l2-messaging/solidity/out/MockStarknetMessaging.sol/MockStarknetMessaging.json ../crates/starknet-devnet-core/contracts/l1-l2-artifacts/

## CAIRO
scarb --manifest-path ./l1-l2-messaging/cairo/Scarb.toml build
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2.contract_class.json ./l1-l2-artifacts/cairo_l1_l2.contract_class.sierra
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2_lib.contract_class.json ./l1-l2-artifacts/cairo_l1_l2_lib.contract_class.sierra
