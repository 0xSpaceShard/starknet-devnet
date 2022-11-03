%lang starknet

from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.cairo_builtins import HashBuiltin
from starkware.cairo.common.math import assert_nn
from starkware.starknet.common.messages import send_message_to_l1

const MESSAGE_WITHDRAW = 0;

// A mapping from a user (L1 Ethereum address) to their balance.
@storage_var
func balance(user: felt) -> (res: felt) {
}

@view
func get_balance{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(user: felt) -> (
    balance: felt
) {
    let (res) = balance.read(user=user);
    return (res,);
}

@external
func increase_balance{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    user: felt, amount: felt
) {
    let (res) = balance.read(user=user);
    balance.write(user, res + amount);
    return ();
}

@external
func withdraw{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    user: felt, amount: felt, L1_CONTRACT_ADDRESS: felt
) {
    // Make sure 'amount' is positive.
    assert_nn(amount);

    let (res) = balance.read(user=user);
    tempvar new_balance = res - amount;

    // Make sure the new balance will be positive.
    assert_nn(new_balance);

    // Update the new balance.
    balance.write(user, new_balance);

    // Send the withdrawal message.
    let (message_payload: felt*) = alloc();
    assert message_payload[0] = MESSAGE_WITHDRAW;
    assert message_payload[1] = user;
    assert message_payload[2] = amount;
    send_message_to_l1(to_address=L1_CONTRACT_ADDRESS, payload_size=3, payload=message_payload);

    return ();
}

@event
func l1_handler_test_event(user: felt, new_balance: felt) {
}

@l1_handler
func deposit{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    from_address: felt, user: felt, amount: felt
) {
    // In a real case scenario, here we would assert from_address value

    // Read the current balance.
    let (res) = balance.read(user=user);

    // Compute and update the new balance.
    tempvar new_balance = res + amount;
    balance.write(user, new_balance);
    l1_handler_test_event.emit(user, new_balance);
    return ();
}
