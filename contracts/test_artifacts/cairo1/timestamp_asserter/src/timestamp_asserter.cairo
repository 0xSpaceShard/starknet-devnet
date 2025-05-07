#[starknet::contract]
pub mod TimestampAsserter {
    use core::starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use core::starknet::get_execution_info;


    #[constructor]
    fn constructor(ref self: ContractState, lock_interval: u64) {
        let current_time: u64 = get_execution_info().block_info.block_timestamp;
        self.deployment_time.write(current_time);
        self.lock_interval.write(lock_interval);
    }

    #[storage]
    pub struct Storage {
        deployment_time: u64,
        lock_interval: u64,
    }

    #[external(v0)]
    pub fn check_time(ref self: ContractState) {
        let current_time: u64 = get_execution_info().block_info.block_timestamp;
        let unlock_time: u64 = self.deployment_time.read() + self.lock_interval.read();
        assert(current_time >= unlock_time, 'Wait a bit more');
    }
}