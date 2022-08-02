"""
Tests RPC blocks
"""

from test.shared import GENESIS_BLOCK_NUMBER, INCORRECT_GENESIS_BLOCK_HASH
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG

from .rpc_utils import rpc_call, get_block_with_transaction, pad_zero, gateway_call


def test_get_block_by_number(deploy_info):
    """
    Get block by number
    """
    gateway_block: dict = get_block_with_transaction(
        deploy_info["transaction_hash"])
    block_hash: str = gateway_block["block_hash"]
    block_number: int = gateway_block["block_number"]
    new_root: str = gateway_block["state_root"]

    resp = rpc_call(
        "starknet_getBlockByNumber", params={"block_number": block_number}
    )
    block = resp["result"]
    transaction_hash: str = pad_zero(deploy_info["transaction_hash"])

    assert block["block_hash"] == pad_zero(block_hash)
    assert block["parent_hash"] == pad_zero(gateway_block["parent_block_hash"])
    assert block["block_number"] == block_number
    assert block["status"] == "ACCEPTED_ON_L2"
    assert block["sequencer_address"] == hex(
        DEFAULT_GENERAL_CONFIG.sequencer_address)
    assert block["new_root"] == pad_zero(new_root)
    assert block["transactions"] == [transaction_hash]


# pylint: disable=unused-argument
def test_get_block_by_number_raises_on_incorrect_number(deploy_info):
    """
    Get block by incorrect number
    """
    ex = rpc_call(
        "starknet_getBlockByNumber", params={"block_number": 1234}
    )

    assert ex["error"] == {
        "code": 26,
        "message": "Invalid block number"
    }


def test_get_block_by_hash(deploy_info):
    """
    Get block by hash
    """
    gateway_block: dict = get_block_with_transaction(
        deploy_info["transaction_hash"])
    block_hash: str = gateway_block["block_hash"]
    new_root: str = gateway_block["state_root"]
    transaction_hash: str = pad_zero(deploy_info["transaction_hash"])

    resp = rpc_call(
        "starknet_getBlockByHash", params={"block_hash": block_hash}
    )
    block = resp["result"]

    assert block["block_hash"] == pad_zero(block_hash)
    assert block["parent_hash"] == pad_zero(gateway_block["parent_block_hash"])
    assert block["block_number"] == gateway_block["block_number"]
    assert block["status"] == "ACCEPTED_ON_L2"
    assert block["sequencer_address"] == hex(
        DEFAULT_GENERAL_CONFIG.sequencer_address)
    assert block["new_root"] == pad_zero(new_root)
    assert block["transactions"] == [transaction_hash]


def test_get_block_by_hash_full_txn_scope(deploy_info):
    """
    Get block by hash with scope FULL_TXNS
    """
    block_hash: str = get_block_with_transaction(
        deploy_info["transaction_hash"])["block_hash"]
    transaction_hash: str = pad_zero(deploy_info["transaction_hash"])
    contract_address: str = pad_zero(deploy_info["address"])

    resp = rpc_call(
        "starknet_getBlockByHash",
        params={
            "block_hash": block_hash,
            "requested_scope": "FULL_TXNS"
        }
    )
    block = resp["result"]

    assert block["transactions"] == [{
        "txn_hash": transaction_hash,
        "max_fee": "0x0",
        "contract_address": contract_address,
        "calldata": [],
        "entry_point_selector": None,
        "signature": [],
        "version": "0x0"
    }]


def test_get_block_by_hash_full_txn_and_receipts_scope(deploy_info):
    """
    Get block by hash with scope FULL_TXN_AND_RECEIPTS
    """
    block_hash: str = get_block_with_transaction(
        deploy_info["transaction_hash"])["block_hash"]
    transaction_hash: str = pad_zero(deploy_info["transaction_hash"])
    contract_address: str = pad_zero(deploy_info["address"])

    resp = rpc_call(
        "starknet_getBlockByHash",
        params={
            "block_hash": block_hash,
            "requested_scope": "FULL_TXN_AND_RECEIPTS"
        }
    )
    block = resp["result"]

    assert block["transactions"] == [{
        "txn_hash": transaction_hash,
        "max_fee": "0x0",
        "contract_address": contract_address,
        "calldata": [],
        "entry_point_selector": None,
        "signature": [],
        "version": "0x0",
        "actual_fee": "0x0",
        "status": "ACCEPTED_ON_L2",
        "statusData": None,
    }]


def test_get_block_by_hash_raises_on_incorrect_hash(deploy_info):
    """
    Get block by incorrect hash
    """
    ex = rpc_call(
        "starknet_getBlockByHash", params={"block_hash": INCORRECT_GENESIS_BLOCK_HASH}
    )

    assert ex["error"] == {
        "code": 24,
        "message": "Invalid block hash"
    }


def test_get_block_transaction_count_by_hash(deploy_info):
    """
    Get count of transactions in block by block hash
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    block_hash: str = block["block_hash"]

    resp = rpc_call(
        "starknet_getBlockTransactionCountByHash", params={"block_hash": block_hash}
    )
    count = resp["result"]

    assert count == 1


def test_get_block_transaction_count_by_hash_raises_on_incorrect_hash(deploy_info):
    """
    Get count of transactions in block by incorrect block hash
    """
    ex = rpc_call(
        "starknet_getBlockTransactionCountByHash", params={"block_hash": INCORRECT_GENESIS_BLOCK_HASH}
    )

    assert ex["error"] == {
        "code": 24,
        "message": "Invalid block hash"
    }


def test_get_block_transaction_count_by_number(deploy_info):
    """
    Get count of transactions in block by block number
    """
    block_number: int = GENESIS_BLOCK_NUMBER + 1

    resp = rpc_call(
        "starknet_getBlockTransactionCountByNumber", params={"block_number": block_number}
    )
    count = resp["result"]

    assert count == 1


def test_get_block_transaction_count_by_number_raises_on_incorrect_number(deploy_info):
    """
    Get count of transactions in block by incorrect block number
    """
    ex = rpc_call(
        "starknet_getBlockTransactionCountByNumber", params={"block_number": 99999}
    )

    assert ex["error"] == {
        "code": 26,
        "message": "Invalid block number"
    }


def test_get_block_number(deploy_info):
    """
    Get the number of the latest accepted  block
    """

    latest_block = gateway_call("get_block", blockNumber="latest")
    latest_block_number: int = latest_block["block_number"]

    resp = rpc_call(
        "starknet_blockNumber", params={}
    )
    block_number: int = resp["result"]

    assert latest_block_number == block_number
