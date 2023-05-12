#!/bin/bash

set -eu

TEST_DIRECTORY=test/contracts
ARTIFACTS_DIRECTORY=test/artifacts

# delete all artifacts
rm -rf $ARTIFACTS_DIRECTORY

# create artifacts directory
mkdir -p $ARTIFACTS_DIRECTORY

# check now for cairo 1 compiler manifest variable to prevent failing after a minute of compilation
echo "Using Cairo 1 compiler at $CAIRO_1_COMPILER_MANIFEST"

# compile Solidity test contracts first
# order matters as Hardhat will remove Cairo artifacts
echo "Compiling Solidity contracts with Hardhat $(npx hardhat --version)"
npx hardhat compile

# compile Cairo test contracts
echo "Compiling Cairo contracts with $(poetry run starknet-compile-deprecated --version)"

number_of_contracts=0
for contract in "$TEST_DIRECTORY"/cairo/*.cairo; do
    basename=$(basename "$contract")

    # create contract directory
    directory="$ARTIFACTS_DIRECTORY/contracts/cairo/${basename}"
    mkdir -p "$directory"

    output=$directory/"${basename%.*}.json"
    abi=$directory/"${basename%.*}_abi.json"

    poetry run starknet-compile-deprecated --output "$output" --abi "$abi" "$contract"
    number_of_contracts=$((number_of_contracts + 1))
done

echo "Compiled $number_of_contracts Cairo files successfully"

./scripts/compile_cairo1.sh
