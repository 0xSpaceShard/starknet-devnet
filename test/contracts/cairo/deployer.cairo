%lang starknet

from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.cairo_builtins import HashBuiltin
from starkware.starknet.common.syscalls import deploy
from starkware.cairo.common.bool import TRUE

// Define a storage variable for the salt.
@storage_var
func salt() -> (value: felt) {
}

@storage_var
func deployable_class_hash() -> (value: felt) {
}

@event
func contract_deployed(contract_address: felt) {
}

@constructor
func constructor{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    class_hash: felt, initial_balance: felt
) {
    deployable_class_hash.write(value=class_hash);

    // deploy an instance from the constructor
    deploy_contract(initial_balance);
    return ();
}

@external
func deploy_contract{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}(
    initial_balance: felt
) {
    let (current_salt) = salt.read();
    let (class_hash) = deployable_class_hash.read();
    let (contract_address) = deploy(
        class_hash=class_hash,
        contract_address_salt=current_salt,
        constructor_calldata_size=1,
        constructor_calldata=cast(new (initial_balance,), felt*),
        deploy_from_zero=TRUE,
    );
    salt.write(value=current_salt + 1);

    contract_deployed.emit(contract_address=contract_address);
    return ();
}
