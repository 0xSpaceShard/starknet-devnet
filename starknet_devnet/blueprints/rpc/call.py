"""
RPC call endpoint
"""

from typing import List

from starkware.starkware_utils.error_handling import StarkException

from starknet_devnet.blueprints.rpc.utils import rpc_felt, assert_block_id_is_latest
from starknet_devnet.blueprints.rpc.structures.payloads import (
    make_invoke_function,
    FunctionCall,
)
from starknet_devnet.blueprints.rpc.structures.types import Felt, BlockId, RpcError
from starknet_devnet.state import state
from starknet_devnet.util import StarknetDevnetException


async def call(request: FunctionCall, block_id: BlockId) -> List[Felt]:
    """
    Call a starknet function without creating a StarkNet transaction
    """
    assert_block_id_is_latest(block_id)

    if not state.starknet_wrapper.contracts.is_deployed(
        int(request["contract_address"], 16)
    ):
        raise RpcError(code=20, message="Contract not found")

    try:
        result = await state.starknet_wrapper.call(
            transaction=make_invoke_function(request)
        )
        result = [rpc_felt(int(res, 16)) for res in result["result"]]
        return result
    except StarknetDevnetException as ex:
        raise RpcError(code=-1, message=ex.message) from ex
    except StarkException as ex:
        if f"Entry point {request['entry_point_selector']} not found" in ex.message:
            raise RpcError(code=21, message="Invalid message selector") from ex
        if "While handling calldata" in ex.message:
            raise RpcError(code=22, message="Invalid call data") from ex
        raise RpcError(code=-1, message=ex.message) from ex
