"""
RPC call endpoint
"""

from typing import Any, List

from starkware.starkware_utils.error_handling import StarkException

from starknet_devnet.blueprints.rpc.schema import validate_schema
from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcFunctionCall,
    make_call_function,
)
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockId,
    Felt,
    PredefinedRpcErrorCode,
    RpcError,
)
from starknet_devnet.blueprints.rpc.utils import (
    assert_block_id_is_valid,
    gateway_felt,
    rpc_felt,
)
from starknet_devnet.state import state


def _validate_calldata(calldata: List[Any]):
    for calldata_value in calldata:
        try:
            int(calldata_value, 16)
        except (ValueError, TypeError) as error:
            raise RpcError.from_spec_name("INVALID_CALL_DATA") from error


@validate_schema("call")
async def call(request: RpcFunctionCall, block_id: BlockId) -> List[Felt]:
    """
    Call a starknet function without creating a Starknet transaction
    """
    await assert_block_id_is_valid(block_id)

    if not await state.starknet_wrapper.is_deployed(
        int(request["contract_address"], 16)
    ):
        raise RpcError.from_spec_name("CONTRACT_NOT_FOUND")

    _validate_calldata(request["calldata"])
    try:
        result = await state.starknet_wrapper.call(
            transaction=make_call_function(request),
            block_id=block_id,
        )
        return [rpc_felt(res) for res in result["result"]]
    except StarkException as ex:
        if ex.code.name == "TRANSACTION_FAILED" and ex.code.value == 39:
            raise RpcError.from_spec_name("CONTRACT_ERROR") from ex
        if (
            f"Entry point {gateway_felt(request['entry_point_selector'])} not found"
            in ex.message
        ):
            raise RpcError.from_spec_name("CONTRACT_ERROR") from ex
        if "While handling calldata" in ex.message:
            raise RpcError.from_spec_name("CONTRACT_ERROR") from ex
        raise RpcError(
            code=PredefinedRpcErrorCode.INTERNAL_ERROR.value, message=ex.message
        ) from ex
