#[starknet::interface]
trait ITimeContract<TContractState> {
   fn get_timestamp(self: @TContractState) -> u64;
}

#[starknet::contract]
mod TimeContract {
    use starknet::get_block_timestamp;

    #[storage]
    struct Storage {
    }

    #[external(v0)]
    impl TimeContract of super::ITimeContract<ContractState> {
        fn get_timestamp(self: @ContractState) -> u64 {
            get_block_timestamp()
        }
    }
}
