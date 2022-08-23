"""
RPC miscellaneous endpoints
"""

from __future__ import annotations

from typing import Union

from starknet_devnet.blueprints.rpc.structures.payloads import Felt, Address
from starknet_devnet.state import state


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


# pylint: disable=unused-argument
# pylint: disable=redefined-builtin
async def get_events(filter: dict) -> dict:
    """
    Returns all events matching the given filter
    """
    raise NotImplementedError()


# pylint: disable=unused-argument
async def get_nonce(contract_address: Address) -> Felt:
    """
    Get the latest nonce associated with the given address
    """
    raise NotImplementedError()
