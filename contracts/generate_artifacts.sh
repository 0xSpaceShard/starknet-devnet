#!/bin/bash
set -eu

# Bash script to generate the artifacts required by Devnet to compile.
mkdir -p l1-l2-artifacts

# L1-L2 messaging:
## SOLIDITY
forge build --root ./l1-l2-messaging/solidity
cp ./l1-l2-messaging/solidity/out/MockStarknetMessaging.sol/MockStarknetMessaging.json ./l1-l2-artifacts/
cp ./l1-l2-messaging/solidity/out/L1L2.sol/L1L2Example.json ./l1-l2-artifacts/

## CAIRO
scarb --manifest-path ./l1-l2-messaging/cairo/Scarb.toml build
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2.contract_class.json ./l1-l2-artifacts/
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2_lib.contract_class.sierra ./l1-l2-artifacts/

cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2.contract_class.json ../crates/starknet-devnet/test_data/cairo1/messaging/cairo_1_l1l2.sierra
cp ./l1-l2-messaging/cairo/target/dev/cairo_l1_l2_lib.contract_class.sierra ../crates/starknet-devnet/test_data/cairo1/messaging/cairo_1_l1l2_lib.sierra
