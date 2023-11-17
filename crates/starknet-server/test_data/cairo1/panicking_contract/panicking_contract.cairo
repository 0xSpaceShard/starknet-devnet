#[starknet::interface]
trait IPanickingContract<TContractState> {
   fn create_panic(self: @TContractState, panic_reason: felt252);
}

#[starknet::contract]
mod PanickingContract {
   #[storage]
   struct Storage {}

   #[external(v0)]
   impl PanickingContract of super::IPanickingContract<ContractState> {
      fn create_panic(self: @ContractState, panic_reason: felt252) {
         panic_with_felt252(panic_reason);
      }
   }
}
