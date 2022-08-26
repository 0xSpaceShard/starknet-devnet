"""
RPC classes endpoints
"""

from starknet_devnet.blueprints.rpc.utils import assert_block_id_is_latest, rpc_felt
from starknet_devnet.blueprints.rpc.structures.payloads import rpc_contract_class
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockId,
    Address,
    Felt,
    RpcError,
)
from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException


async def get_class(class_hash: Felt) -> dict:
    """
    Get the contract class definition associated with the given hash
    """
    try:
        result = state.starknet_wrapper.contracts.get_class_by_hash(
            class_hash=int(class_hash, 16)
        )
    except StarknetDevnetException as ex:
        raise RpcError(
            code=28, message="The supplied contract class hash is invalid or unknown"
        ) from ex

    return rpc_contract_class(result)


async def get_class_hash_at(block_id: BlockId, contract_address: Address) -> Felt:
    """
    Get the contract class hash in the given block for the contract deployed at the given address
    """
    assert_block_id_is_latest(block_id)

    try:
        result = state.starknet_wrapper.contracts.get_class_hash_at(
            address=int(contract_address, 16)
        )
    except StarknetDevnetException as ex:
        raise RpcError(
            code=28, message="The supplied contract class hash is invalid or unknown"
        ) from ex

    return rpc_felt(result)


async def get_class_at(block_id: BlockId, contract_address: Address) -> dict:
    """
    Get the contract class definition in the given block at the given address
    """
    assert_block_id_is_latest(block_id)

    try:
        class_hash = state.starknet_wrapper.contracts.get_class_hash_at(
            address=int(contract_address, 16)
        )
        result = state.starknet_wrapper.contracts.get_class_by_hash(
            class_hash=class_hash
        )
    except StarknetDevnetException as ex:
        raise RpcError(code=20, message="Contract not found") from ex

    return rpc_contract_class(result)
