"""
RPC storage endpoints
"""
from starknet_devnet.blueprints.rpc.schema import validate_schema
from starknet_devnet.blueprints.rpc.structures.types import (
    Address,
    BlockId,
    Felt,
    RpcError,
)
from starknet_devnet.blueprints.rpc.utils import assert_block_id_is_valid, rpc_felt
from starknet_devnet.state import state


@validate_schema("getStorageAt")
async def get_storage_at(
    contract_address: Address, key: str, block_id: BlockId
) -> Felt:
    """
    Get the value of the storage at the given address and key
    """
    await assert_block_id_is_valid(block_id)

    if not await state.starknet_wrapper.is_deployed(int(contract_address, 16)):
        raise RpcError.from_spec_name("CONTRACT_NOT_FOUND")

    storage = await state.starknet_wrapper.get_storage_at(
        contract_address=int(contract_address, 16),
        key=int(key, 16),
        block_id=block_id,
    )
    return rpc_felt(storage)
