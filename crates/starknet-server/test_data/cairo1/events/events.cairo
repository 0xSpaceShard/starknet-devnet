use core::debug::PrintTrait;
use core::traits::Into;
use core::result::ResultTrait;
use starknet::syscalls::{deploy_syscall, get_block_hash_syscall};
use traits::TryInto;
use option::OptionTrait;
use starknet::SyscallResultTrait;
use starknet::class_hash::Felt252TryIntoClassHash;
use array::ArrayTrait;
use array::SpanTrait;

#[starknet::interface]
trait IContractWithEvent<T> {
    fn emit_event(ref self: T, incremental: bool);
}

#[starknet::contract]
mod contract_with_event {
    use traits::Into;
    use starknet::info::get_contract_address;
    #[storage]
    struct Storage {
        value: u128,
    }

    #[derive(Copy, Drop, PartialEq, starknet::Event)]
    struct IncrementalEvent {
        value: u128,
    }

    #[derive(Copy, Drop, PartialEq, starknet::Event)]
    struct StaticEvent {}

    #[event]
    #[derive(Copy, Drop, PartialEq, starknet::Event)]
    enum Event {
        IncrementalEvent: IncrementalEvent,
        StaticEvent: StaticEvent,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.value.write(0);
    }

    #[external(v0)]
    fn emit_event(ref self: ContractState, incremental: bool) {
        if incremental {
            self.emit(Event::IncrementalEvent(IncrementalEvent { value: self.value.read() }));
            self.value.write(self.value.read() + 1);
        } else {
            self.emit(Event::StaticEvent(StaticEvent {}));
        }
    }
}

use contract_with_event::{Event, IncrementalEvent, StaticEvent};