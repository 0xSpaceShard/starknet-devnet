"""
RPC utilities
"""

from starknet_devnet.blueprints.rpc.structures.types import BlockId, RpcError, Felt
from starknet_devnet.util import StarknetDevnetException
from starknet_devnet.state import state


def block_tag_to_block_number(block_id: BlockId) -> BlockId:
    """
    Changes block_id from tag to dict with "block_number" field
    """
    if isinstance(block_id, str):
        if block_id == "latest":
            return {
                "block_number": state.starknet_wrapper.blocks.get_number_of_blocks() - 1
            }

        if block_id == "pending":
            raise RpcError(
                code=-1,
                message="Calls with block_id == 'pending' are not supported currently.",
            )

        raise RpcError(code=24, message="Invalid block id")

    return block_id


def get_block_by_block_id(block_id: BlockId) -> dict:
    """
    Get block using different method depending on block_id type
    """
    if block_id in ["latest", "pending"]:
        block_id = block_tag_to_block_number(block_id)

    try:
        if "block_hash" in block_id:
            return state.starknet_wrapper.blocks.get_by_hash(
                block_hash=block_id["block_hash"]
            )
        return state.starknet_wrapper.blocks.get_by_number(
            block_number=block_id["block_number"]
        )
    except StarknetDevnetException as ex:
        raise RpcError(code=24, message="Invalid block id") from ex


def assert_block_id_is_latest(block_id: BlockId) -> None:
    """
    Assert block_id is "latest" and throw RpcError otherwise
    """
    if block_id != "latest":
        raise RpcError(
            code=-1,
            message="Calls with block_id != 'latest' are not supported currently.",
        )


def rpc_felt(value: int) -> Felt:
    """
    Convert integer to 0x0 prefixed felt
    """
    if value == 0:
        return "0x00"
    return "0x0" + hex(value).lstrip("0x")


def pad_zero(felt: str) -> Felt:
    """
    Convert felt with format `0xValue` to format `0x0Value`
    """
    if felt == "0x0":
        return "0x00"
    return "0x0" + felt.lstrip("0x")


def rpc_root(root: str) -> Felt:
    """
    Convert 0 prefixed root to 0x prefixed root
    """
    return "0x0" + (root.lstrip("0") or "0")


def rpc_response(message_id: int, content: dict) -> dict:
    """
    Wrap response content in rpc format
    """
    return {"jsonrpc": "2.0", "id": message_id, "result": content}


def rpc_error(message_id: int, code: int, message: str) -> dict:
    """
    Wrap error in rpc format
    """
    return {
        "jsonrpc": "2.0",
        "id": message_id,
        "error": {"code": code, "message": message},
    }
