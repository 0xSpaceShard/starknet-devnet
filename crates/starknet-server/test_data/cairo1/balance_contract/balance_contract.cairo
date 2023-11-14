#[starknet::interface]
trait IBalanceContract<TContractState> {
   fn increase_balance(ref self: TContractState, amount1: u128, amount2: u128);
   fn get_balance(self: @TContractState) -> u128;
   fn create_panic(self: @TContractState, panic_reason: felt252);
}

#[starknet::contract]
mod BalanceContract {

   #[storage]
   struct Storage {
      balance: u128,
   }

   #[event]
   #[derive(Drop, starknet::Event)]
    enum Event {
        BalanceIncreased: BalanceIncreased
    }

    #[derive(Drop, starknet::Event)]
    struct BalanceIncreased {
        amount: u128
    }

   #[constructor]
   fn constructor(ref self: ContractState, initial_balance: u128) {
      self.balance.write(initial_balance);
   }

   #[external(v0)]
   impl BalanceContract of super::IBalanceContract<ContractState> {
      fn get_balance(self: @ContractState) -> u128 {
         self.balance.read()
      }

      fn increase_balance(ref self: ContractState, amount1: u128, amount2: u128) {
         let current = self.balance.read();
         self.balance.write(current + amount1 + amount2);
         self.emit(Event::BalanceIncreased(BalanceIncreased { amount: amount1 + amount2 }));
      }

      fn create_panic(self: @ContractState, panic_reason: felt252) {
         panic_with_felt252(panic_reason);
      }
   }
}
