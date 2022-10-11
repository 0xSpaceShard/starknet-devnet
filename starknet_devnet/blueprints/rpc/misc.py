"""
RPC miscellaneous endpoints
"""

from __future__ import annotations
from itertools import islice
from queue import Empty
from typing import Union, List
import collections
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockId,
    Felt,
    Address,
)
from starknet_devnet.blueprints.rpc.structures.responses import (
    RpcEventsResult,
)

from starknet_devnet.state import state


def is_empty(value):
    """
    Return true if empty.
    """
    return not value and isinstance(value, collections.Iterable)


def is_not_empty(value):
    """
    Return true if not empty.
    """
    return not is_empty(value)


def filter_address(address, event):
    """
    Filter by address.
    """
    return bool(address == "" or event.from_address == int(address, 0))


def filter_keys(keys, event):
    """
    Filter by keys.
    """
    return bool(
        (is_empty(keys)) or (is_not_empty(keys) and bool(set(event.keys) & set(keys)))
    )


def get_events_from_block(block, address, keys):
    """
    Return filtered events.
    """
    events = []
    for event in [e for r in block.transaction_receipts for e in r.events]:
        if filter_keys(keys, event) and filter_address(address, event):
            events.append(event)

    return events


async def chain_id() -> str:
    """
    Return the currently configured StarkNet chain id
    """
    devnet_state = state.starknet_wrapper.get_state()
    config = devnet_state.general_config
    chain: int = config.chain_id.value
    return hex(chain)


async def syncing() -> Union[dict, bool]:
    """
    Returns an object about the sync status, or false if the node is not synching
    """
    return False


# pylint: disable=too-many-arguments
async def get_events(
    from_block: BlockId,
    to_block: BlockId,
    address: Address = "",
    keys: List[Address] = None,
    chunk_size: int = 0,
    continuation_token: str = "",
) -> str:
    """
    Returns all events matching the given filters.

    In our implementation continuation_token is just a number.

    In state.starknet_wrapper.get_state().events there is no relation between blocks.
    This is why we need to iterate block by block, take all events,
    and chunk it later which is not an optimal solution.
    """
    events = []
    keys = [] if keys is None else [int(k, 0) for k in keys]
    to_block = (
        state.starknet_wrapper.blocks.get_number_of_blocks()
        if to_block == "latest"
        else to_block
    )

    for block_number in range(int(from_block), int(to_block)):
        block = state.starknet_wrapper.blocks.get_by_number(block_number)
        if block.transaction_receipts != ():
            events.extend(get_events_from_block(block, address, keys))

    if chunk_size > 0 and continuation_token == "":
        events = events[:chunk_size]
    elif chunk_size > 0 and continuation_token != "":
        events = (list(islice(events, chunk_size * (int(continuation_token)), None)))[
            :chunk_size
        ]

    return RpcEventsResult(
        events=events,
        continuation_token=continuation_token,
    )


async def get_nonce(contract_address: Address) -> Felt:
    """
    Get the latest nonce associated with the given address
    """
    raise NotImplementedError()
