%lang starknet

from starkware.cairo.common.cairo_builtins import HashBuiltin
from starkware.starknet.common.syscalls import get_block_number

# Define a storage variable.
@storage_var
func block_number() -> (res : felt):
end

@view
func my_get_block_number{
        syscall_ptr : felt*,
        pedersen_ptr : HashBuiltin*,
        range_check_ptr
    }() -> (ts: felt):
    let (ts) = get_block_number()
    return (ts)
end

@external
func write_block_number{
        syscall_ptr : felt*, 
        pedersen_ptr : HashBuiltin*,
        range_check_ptr
    }():
    let (fetched) = get_block_number()
    block_number.write(fetched)
    return ()
end

@view
func read_block_number{
        syscall_ptr : felt*, 
        pedersen_ptr : HashBuiltin*,
        range_check_ptr
    }() -> (block_number : felt):
    let (fetched) = block_number.read()
    return (block_number=fetched)
end

@external
func fail{
    syscall_ptr : felt*, 
    pedersen_ptr : HashBuiltin*,
    range_check_ptr
    }():
        assert 1 = 2
        return ()
end
