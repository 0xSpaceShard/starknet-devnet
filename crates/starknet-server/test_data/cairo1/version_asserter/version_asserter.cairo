#[starknet::interface]
trait IVersionAsserter<TContractState> {
   fn assert_version(self: @TContractState, expected_version: felt252);
}

#[starknet::contract]
mod VersionAsserter {
   use starknet::syscalls::get_execution_info_syscall;
   use starknet::SyscallResultTrait;

   #[storage]
   struct Storage {}

   #[external(v0)]
   impl VersionAsserter of super::IVersionAsserter<ContractState> {
      fn assert_version(self: @ContractState, expected_version: felt252) {
         let exec_info = get_execution_info_syscall().unwrap_syscall().unbox();
         let tx_info = exec_info.tx_info.unbox();
         let version = tx_info.version;
         assert(version == expected_version, 'Version should be equal');
      }
   }
}
