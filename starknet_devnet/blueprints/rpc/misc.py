"""
RPC miscellaneous endpoints
"""

from __future__ import annotations

from typing import List, Union

from starkware.starknet.services.api.feeder_gateway.response_objects import (
    LATEST_BLOCK_ID,
    PENDING_BLOCK_ID,
    BlockStatus,
    StarknetBlock,
)

from starknet_devnet.blueprints.rpc.schema import validate_schema
from starknet_devnet.blueprints.rpc.structures.responses import (
    EmittedEvent,
    RpcEventsResult,
    RpcEventsResultWithoutContinuationToken,
)
from starknet_devnet.blueprints.rpc.structures.types import (
    Address,
    BlockId,
    Felt,
    PredefinedRpcErrorCode,
    RpcError,
)
from starknet_devnet.blueprints.rpc.utils import (
    assert_block_id_is_valid,
    get_block_by_block_id,
    rpc_felt,
)
from starknet_devnet.state import state


def check_address(address, event):
    """
    Check address.
    """
    return event.from_address == int(address, 0)


def check_keys(keys, event):
    """
    Check keys.
    """
    return bool(set(event.keys) & set(keys))


def _get_events_from_block(block: StarknetBlock, address, keys):
    """
    Return filtered events.
    """
    events = []
    for receipt, event in [
        (r, e) for r in block.transaction_receipts for e in r.events
    ]:
        if check_keys(keys, event) and check_address(address, event):
            _event: EmittedEvent = {
                "from_address": rpc_felt(event.from_address),
                "keys": [rpc_felt(e) for e in event.keys],
                "data": [rpc_felt(d) for d in event.data],
                # hash and number defaulting to 0 if None (if pending)
                "block_hash": rpc_felt(block.block_hash or "0x0"),
                "block_number": block.block_number or 0,
                "transaction_hash": rpc_felt(receipt.transaction_hash),
            }
            events.append(_event)

    return events


@validate_schema("chainId")
async def chain_id() -> str:
    """
    Return the currently configured Starknet chain id
    """
    devnet_state = state.starknet_wrapper.get_state()
    config = devnet_state.general_config
    chain: int = config.chain_id.value
    return hex(chain)


@validate_schema("syncing")
async def syncing() -> Union[dict, bool]:
    """
    Returns an object about the sync status, or false if the node is not synching
    """
    return False


async def _get_events_range(
    from_block: StarknetBlock, to_block: StarknetBlock
) -> List[BlockId]:
    if from_block.status == BlockStatus.PENDING:
        # if from_block is pending, then to_block ought to be as well
        if to_block.status == BlockStatus.PENDING:
            return [PENDING_BLOCK_ID]
        return []

    include_pending = to_block.status == BlockStatus.PENDING
    if to_block.status == BlockStatus.PENDING:
        to_block = await get_block_by_block_id(LATEST_BLOCK_ID)

    events_range = list(range(from_block.block_number, to_block.block_number + 1))
    if include_pending:
        events_range.append(PENDING_BLOCK_ID)

    return events_range


# pylint: disable=redefined-builtin
# filter name is determined by current RPC implementation and starknet specification
@validate_schema("getEvents")
async def get_events(
    filter,
) -> Union[RpcEventsResult, RpcEventsResultWithoutContinuationToken]:
    """
    Returns all events matching the given filters.

    In our implementation continuation_token is just a number.

    In state.starknet_wrapper.get_state().events there is no relation between blocks.
    This is why we need to iterate block by block, take all events,
    and chunk it later which is not an optimal solution.
    """
    # Required parameters
    from_block = await get_block_by_block_id(filter.get("from_block"))
    to_block = await get_block_by_block_id(filter.get("to_block"))
    block_range = await _get_events_range(from_block, to_block)

    try:
        chunk_size = int(filter.get("chunk_size"))
    except ValueError as ex:
        raise RpcError(
            code=PredefinedRpcErrorCode.INVALID_PARAMS.value,
            message=f"invalid chunk_size: '{filter.get('chunk_size')}'",
        ) from ex

    address = filter.get("address")
    keys = [int(k, 0) for k in filter.get("keys")]
    # Optional parameter
    continuation_token = int(filter.get("continuation_token", "0"))

    events = []

    for block_number in block_range:
        block = await state.starknet_wrapper.blocks.get_by_number(block_number)
        if block.transaction_receipts:
            events.extend(_get_events_from_block(block, address, keys))

    # Chunking
    start_index = continuation_token * chunk_size
    chunked_events = events[start_index : start_index + chunk_size]
    remaining_events_length = len(events) - start_index

    # Continuation_token should be increased only if events are not empty
    if remaining_events_length > chunk_size:
        continuation_token = continuation_token + 1
        return RpcEventsResult(
            events=chunked_events, continuation_token=str(continuation_token)
        )

    return RpcEventsResultWithoutContinuationToken(events=chunked_events)


@validate_schema("getNonce")
async def get_nonce(block_id: BlockId, contract_address: Address) -> Felt:
    """
    Get the nonce associated with the given address in the given block
    """
    await assert_block_id_is_valid(block_id)

    if not await state.starknet_wrapper.is_deployed(int(contract_address, 16)):
        raise RpcError.from_spec_name("CONTRACT_NOT_FOUND")

    result = await state.starknet_wrapper.get_nonce(
        contract_address=int(contract_address, 16), block_id=block_id
    )

    return rpc_felt(result)
