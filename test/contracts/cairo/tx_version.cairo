%lang starknet

from starkware.cairo.common.cairo_builtins import HashBuiltin
from starkware.starknet.common.syscalls import get_tx_info

@storage_var
func tx_version() -> (version: felt) {
}

@view
func get_last_tx_version{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}() -> (
    version: felt
) {
    let (version) = tx_version.read();
    return (version,);
}

@view
func get_tx_version{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}() -> (
    version: felt
) {
    let (tx_info) = get_tx_info();
    return (version=tx_info.version);
}

@external
func set_tx_version{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}() {
    let (tx_info) = get_tx_info();

    tx_version.write(tx_info.version);

    return ();
}
