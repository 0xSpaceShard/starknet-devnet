%lang starknet

from starkware.cairo.common.cairo_builtins import HashBuiltin

@view
func foo{syscall_ptr: felt*, pedersen_ptr: HashBuiltin*, range_check_ptr}() -> (res: felt) {
    return (43,);
}
