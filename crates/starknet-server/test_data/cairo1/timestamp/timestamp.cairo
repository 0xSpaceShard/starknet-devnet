#[starknet::interface]
trait ITimeContract<TContractState> {
    fn get_timestamp(self: @TContractState) -> u64;
    fn get_storage_timestamp(self: @TContractState) -> u64;
    fn set_current_timestamp(ref self: TContractState);
    fn set_storage_timestamp(ref self: TContractState, timestamp: u64);
}

#[starknet::contract]
mod TimeContract {
    use starknet::get_block_timestamp;

    #[storage]
    struct Storage {
        timestamp: u64
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.timestamp.write(get_block_timestamp());
    }

    #[external(v0)]
    impl TimeContract of super::ITimeContract<ContractState> {
        fn get_timestamp(self: @ContractState) -> u64 {
            get_block_timestamp()
        }
        
        fn get_storage_timestamp(self: @ContractState) -> u64 {
            self.timestamp.read()
        }

        fn set_current_timestamp(ref self: ContractState) {
            self.timestamp.write(get_block_timestamp());
        }

        fn set_storage_timestamp(ref self: ContractState, timestamp: u64) {
            self.timestamp.write(timestamp);
        }
    }
}
