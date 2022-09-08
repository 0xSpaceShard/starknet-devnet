"""
Tests RPC blocks
"""
from test.shared import GENESIS_BLOCK_NUMBER, INCORRECT_GENESIS_BLOCK_HASH

import pytest
from starknet_devnet.blueprints.rpc.structures.types import (
    BlockNumberDict,
    BlockHashDict,
    rpc_txn_type,
)
from starknet_devnet.blueprints.rpc.utils import rpc_root
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

from .rpc_utils import rpc_call, pad_zero, gateway_call


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize("block_id", ["hash", "number", "tag"], indirect=True)
def test_get_block_with_tx_hashes(deploy_info, gateway_block, block_id):
    """
    Get block with tx hashes
    """
    block_hash: str = gateway_block["block_hash"]
    block_number: int = gateway_block["block_number"]
    new_root: str = rpc_root(gateway_block["state_root"])

    resp = rpc_call("starknet_getBlockWithTxHashes", params={"block_id": block_id})
    block = resp["result"]
    transaction_hash: str = pad_zero(deploy_info["transaction_hash"])

    assert block == {
        "block_hash": pad_zero(block_hash),
        "parent_hash": pad_zero(gateway_block["parent_block_hash"]),
        "block_number": block_number,
        "status": "ACCEPTED_ON_L2",
        "sequencer_address": pad_zero(hex(DEFAULT_GENERAL_CONFIG.sequencer_address)),
        "new_root": new_root,
        "timestamp": gateway_block["timestamp"],
        "transactions": [transaction_hash],
    }


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
@pytest.mark.parametrize(
    "block_id",
    [
        BlockNumberDict(block_number=1234),
        BlockHashDict(block_hash=pad_zero(INCORRECT_GENESIS_BLOCK_HASH)),
    ],
)
def test_get_block_with_tx_hashes_raises_on_incorrect_block_id(block_id):
    """
    Get block with tx hashes by incorrect block_id
    """
    ex = rpc_call("starknet_getBlockWithTxHashes", params={"block_id": block_id})

    assert ex["error"] == {"code": 24, "message": "Invalid block id"}


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
@pytest.mark.parametrize("block_id", ["hash", "number", "tag"], indirect=True)
def test_get_block_with_txs(gateway_block, block_id):
    """
    Get block with txs by block id
    """
    block_hash: str = gateway_block["block_hash"]
    block_number: int = gateway_block["block_number"]
    new_root: str = rpc_root(gateway_block["state_root"])
    block_tx = gateway_block["transactions"][0]

    resp = rpc_call("starknet_getBlockWithTxs", params={"block_id": block_id})
    block = resp["result"]

    assert block == {
        "block_hash": pad_zero(block_hash),
        "parent_hash": pad_zero(gateway_block["parent_block_hash"]),
        "block_number": block_number,
        "status": "ACCEPTED_ON_L2",
        "sequencer_address": pad_zero(hex(DEFAULT_GENERAL_CONFIG.sequencer_address)),
        "new_root": new_root,
        "timestamp": gateway_block["timestamp"],
        "transactions": [
            {
                "class_hash": pad_zero(block_tx["class_hash"]),
                "constructor_calldata": [
                    pad_zero(data) for data in block_tx["constructor_calldata"]
                ],
                "contract_address": pad_zero(block_tx["contract_address"]),
                "contract_address_salt": pad_zero(block_tx["contract_address_salt"]),
                "transaction_hash": pad_zero(block_tx["transaction_hash"]),
                "type": rpc_txn_type(block_tx["type"]),
                "version": "0x0",
            }
        ],
    }


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
@pytest.mark.parametrize(
    "block_id",
    [
        BlockNumberDict(block_number=1234),
        BlockHashDict(block_hash=pad_zero(INCORRECT_GENESIS_BLOCK_HASH)),
    ],
)
def test_get_block_with_txs_raises_on_incorrect_block_id(block_id):
    """
    Get block with txs by incorrect block_id
    """
    ex = rpc_call("starknet_getBlockWithTxHashes", params={"block_id": block_id})

    assert ex["error"] == {"code": 24, "message": "Invalid block id"}


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info", "gateway_block")
@pytest.mark.parametrize("block_id", ["hash", "number", "tag"], indirect=True)
def test_get_block_transaction_count(block_id):
    """
    Get count of transactions in block by block id
    """
    if "block_number" in block_id:
        block_id["block_number"] = GENESIS_BLOCK_NUMBER + 1

    resp = rpc_call("starknet_getBlockTransactionCount", params={"block_id": block_id})
    count = resp["result"]

    assert count == 1


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
@pytest.mark.parametrize(
    "block_id",
    [
        BlockNumberDict(block_number=99999),
        BlockHashDict(block_hash=pad_zero(INCORRECT_GENESIS_BLOCK_HASH)),
    ],
)
def test_get_block_transaction_count_raises_on_incorrect_block_id(block_id):
    """
    Get count of transactions in block by incorrect block id
    """
    ex = rpc_call("starknet_getBlockTransactionCount", params={"block_id": block_id})

    assert ex["error"] == {"code": 24, "message": "Invalid block id"}


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_get_block_number():
    """
    Get the number of the latest accepted block
    """

    latest_block = gateway_call("get_block", blockNumber="latest")
    latest_block_number: int = latest_block["block_number"]

    resp = rpc_call("starknet_blockNumber", params={})
    block_number: int = resp["result"]

    assert latest_block_number == block_number
