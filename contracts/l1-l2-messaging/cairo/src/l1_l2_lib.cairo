//! L1 L2 messaging contract, aims at being use as a library.

#[starknet::contract]
mod l1_l2_lib {
    const MESSAGE_WITHDRAW: felt252 = 0;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn send_withdraw_message(
        ref self: ContractState, user: felt252, amount: felt252, l1_address: felt252
    ) {
        let payload = array![MESSAGE_WITHDRAW, user, amount,];

        starknet::send_message_to_l1_syscall(l1_address, payload.span()).unwrap();
    }
}
