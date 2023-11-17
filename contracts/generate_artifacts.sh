#!/bin/bash

# Bash script to generate the artifacts required by Devnet to compile.

## SOLIDITY
forge build --root ./solidity
cp ./solidity/out/MockStarknetMessaging.sol/MockStarknetMessaging.json ./artifacts/MockStarknetMessaging.json

## CAIRO
