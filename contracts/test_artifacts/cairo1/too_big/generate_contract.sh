#!/bin/bash

set -euo pipefail

if [ $# -ne 1 ]; then
    echo >&2 "Error! Usage: $0 <N_PROPERTIES>"
    exit 1
fi

N="$1"

echo '#[starknet::contract]'
echo 'pub mod Dummy {'
echo '    use core::starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};'
echo ''
echo '    #[storage]'
echo '    pub struct Storage {'

for i in $(seq 1 $N); do
    echo "        balance_$i: felt252,"
done

echo '    }'
echo ''

for i in $(seq 1 $N); do
    echo '    #[external(v0)]'
    echo "    pub fn increment_balance_$i(ref self: ContractState) {"
    echo "        self.balance_$i.write(self.balance_$i.read() + 1)"
    echo '    }'
    echo ''
    echo '    #[external(v0)]'
    echo "    pub fn get_balance_$i(self: @ContractState) -> felt252 {"
    echo "        self.balance_$i.read()"
    echo '    }'
    echo ''
done

echo '}'
