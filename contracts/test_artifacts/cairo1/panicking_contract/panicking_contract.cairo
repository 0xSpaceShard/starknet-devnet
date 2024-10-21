use core::starknet::ContractAddress;

#[starknet::interface]
trait IPanickingContract<TContractState> {
   fn create_panic(self: @TContractState, panic_reason: felt252);
   fn create_panic_in_another_contract(self: @TContractState, address: ContractAddress, panic_reason: felt252);
}

#[starknet::contract]
mod PanickingContract {
   use core::starknet::{ContractAddress, call_contract_syscall};

   #[storage]
   struct Storage {}

   #[abi(embed_v0)]
   impl PanickingContract of super::IPanickingContract<ContractState> {
      fn create_panic(self: @ContractState, panic_reason: felt252) {
         panic_with_felt252(panic_reason);
      }

      fn create_panic_in_another_contract(self: @ContractState, address: ContractAddress, panic_reason: felt252) {
         call_contract_syscall(
            address,
            selector!("create_panic"),
            array![panic_reason].span()
         ).unwrap();
      }
   }
}
