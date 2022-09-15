"""
Tests RPC transactions
"""

from __future__ import annotations

from typing import List

import pytest

from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcContractClass,
    RpcBroadcastedDeclareTxn,
    EntryPoints,
    RpcBroadcastedDeployTxn,
    RpcBroadcastedInvokeTxnV0,
    RpcBroadcastedInvokeTxnV1,
)
from starknet_devnet.blueprints.rpc.structures.types import rpc_txn_type
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from .rpc_utils import rpc_call, get_block_with_transaction, pad_zero
from ..shared import INCORRECT_GENESIS_BLOCK_HASH, SUPPORTED_TX_VERSION


def pad_zero_entry_points(entry_points: EntryPoints) -> None:
    """
    Pad zero every selector in entry points in contract_class
    """

    def pad_selector(entry_point):
        return {
            "offset": entry_point["offset"],
            "selector": pad_zero(entry_point["selector"]),
        }

    for entry_point_type, entry_point_list in entry_points.items():
        entry_points[entry_point_type] = [
            pad_selector(entry_point) for entry_point in entry_point_list
        ]


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_hash_deploy(deploy_info):
    """
    Get transaction by hash
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = deploy_info["transaction_hash"]
    contract_address: str = deploy_info["address"]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": pad_zero(transaction_hash),
        "class_hash": pad_zero(block_tx["class_hash"]),
        "version": hex(SUPPORTED_TX_VERSION),
        "type": rpc_txn_type(block_tx["type"]),
        "contract_address": pad_zero(contract_address),
        "contract_address_salt": pad_zero(block_tx["contract_address_salt"]),
        "constructor_calldata": ["0x045"],
    }


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_get_transaction_by_hash_invoke(invoke_info):
    """
    Get transaction by hash
    """
    block = get_block_with_transaction(invoke_info["transaction_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = invoke_info["transaction_hash"]
    contract_address: str = invoke_info["address"]
    entry_point_selector: str = invoke_info["entry_point_selector"]
    signature: List[str] = [pad_zero(hex(int(sig))) for sig in invoke_info["signature"]]
    calldata: List[str] = [pad_zero(hex(int(data))) for data in invoke_info["calldata"]]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": pad_zero(transaction_hash),
        "max_fee": pad_zero(block_tx["max_fee"]),
        "version": hex(SUPPORTED_TX_VERSION),
        "signature": signature,
        "nonce": pad_zero(hex(0)),
        "type": rpc_txn_type(block_tx["type"]),
        "contract_address": contract_address,
        "entry_point_selector": pad_zero(entry_point_selector),
        "calldata": calldata,
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_hash_declare(declare_info):
    """
    Get transaction by hash
    """
    block = get_block_with_transaction(declare_info["transaction_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = declare_info["transaction_hash"]
    signature: List[str] = [
        pad_zero(hex(int(sig))) for sig in declare_info["signature"]
    ]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": pad_zero(transaction_hash),
        "max_fee": pad_zero(block_tx["max_fee"]),
        "version": block_tx["version"],
        "signature": signature,
        "nonce": pad_zero(block_tx["nonce"]),
        "type": rpc_txn_type(block_tx["type"]),
        "class_hash": pad_zero(block_tx["class_hash"]),
        "sender_address": pad_zero(block_tx["sender_address"]),
    }


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_get_transaction_by_hash_raises_on_incorrect_hash():
    """
    Get transaction by incorrect hash
    """
    ex = rpc_call("starknet_getTransactionByHash", params={"transaction_hash": "0x00"})

    assert ex["error"] == {"code": 25, "message": "Transaction hash not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_block_id_and_index(deploy_info):
    """
    Get transaction by block id and transaction index
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = deploy_info["transaction_hash"]
    contract_address: str = deploy_info["address"]
    block_number: str = block["block_number"]
    index: int = 0

    resp = rpc_call(
        "starknet_getTransactionByBlockIdAndIndex",
        params={
            "block_id": {
                "block_number": block_number,
            },
            "index": index,
        },
    )
    transaction = resp["result"]

    assert transaction == {
        "class_hash": pad_zero(block_tx["class_hash"]),
        "constructor_calldata": [
            pad_zero(tx) for tx in block_tx["constructor_calldata"]
        ],
        "contract_address": pad_zero(contract_address),
        "contract_address_salt": pad_zero(block_tx["contract_address_salt"]),
        "transaction_hash": pad_zero(transaction_hash),
        "type": rpc_txn_type(block_tx["type"]),
        "version": hex(SUPPORTED_TX_VERSION),
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_block_id_and_index_raises_on_incorrect_block_hash():
    """
    Get transaction by incorrect block id
    """
    ex = rpc_call(
        "starknet_getTransactionByBlockIdAndIndex",
        params={
            "block_id": {"block_hash": pad_zero(INCORRECT_GENESIS_BLOCK_HASH)},
            "index": 0,
        },
    )

    assert ex["error"] == {"code": 24, "message": "Block not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_block_id_and_index_raises_on_incorrect_index(deploy_info):
    """
    Get transaction by block hash and incorrect transaction index
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    block_hash: str = block["block_hash"]

    ex = rpc_call(
        "starknet_getTransactionByBlockIdAndIndex",
        params={
            "block_id": {
                "block_hash": pad_zero(block_hash),
            },
            "index": 999999,
        },
    )

    assert ex["error"] == {
        "code": 27,
        "message": "Invalid transaction index in a block",
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_declare_transaction_receipt(declare_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = declare_info["transaction_hash"]
    block = get_block_with_transaction(transaction_hash)

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    receipt = resp["result"]

    assert receipt == {
        "transaction_hash": pad_zero(transaction_hash),
        "actual_fee": rpc_felt(0),
        "status": "ACCEPTED_ON_L2",
        "block_hash": pad_zero(block["block_hash"]),
        "block_number": block["block_number"],
        "type": "DECLARE",
        "messages_sent": [],
        "events": [],
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_invoke_transaction_receipt(invoke_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = invoke_info["transaction_hash"]

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    receipt = resp["result"]

    # Standard == receipt dict test cannot be done here, because invoke transaction fails since no contracts
    # are actually deployed on devnet, when running test without run_devnet_in_background fixture
    assert receipt["transaction_hash"] == pad_zero(transaction_hash)
    assert receipt["actual_fee"] == rpc_felt(0)
    assert receipt["type"] == "INVOKE"
    assert receipt["events"] == []
    assert receipt["messages_sent"] == []


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_get_transaction_receipt_on_incorrect_hash():
    """
    Get transaction receipt by incorrect hash
    """
    ex = rpc_call(
        "starknet_getTransactionReceipt", params={"transaction_hash": rpc_felt(0)}
    )

    assert ex["error"] == {"code": 25, "message": "Transaction hash not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_deploy_transaction_receipt(deploy_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = deploy_info["transaction_hash"]
    block = get_block_with_transaction(transaction_hash)

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": pad_zero(transaction_hash)},
    )
    receipt = resp["result"]

    assert receipt == {
        "contract_address": pad_zero(deploy_info["address"]),
        "transaction_hash": pad_zero(transaction_hash),
        "actual_fee": rpc_felt(0),
        "status": "ACCEPTED_ON_L2",
        "block_hash": pad_zero(block["block_hash"]),
        "block_number": block["block_number"],
        "type": "DEPLOY",
        "messages_sent": [],
        "events": [],
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_invoke_transaction_v0(invoke_content):
    """
    Add invoke transaction version 0
    """
    invoke_transaction = RpcBroadcastedInvokeTxnV0(
        type=invoke_content["type"],
        max_fee=rpc_felt(0),
        version=hex(0),
        signature=[],
        nonce=rpc_felt(0),
        contract_address=pad_zero(invoke_content["contract_address"]),
        entry_point_selector=pad_zero(invoke_content["entry_point_selector"]),
        calldata=[pad_zero(hex(int(data))) for data in invoke_content["calldata"]],
    )

    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={"invoke_transaction": invoke_transaction},
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == {"transaction_hash"}
    assert receipt["transaction_hash"][:3] == "0x0"


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_invoke_transaction_v1(invoke_content):
    """
    Add invoke transaction version 1
    """
    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type=invoke_content["type"],
        max_fee=rpc_felt(0),
        version=hex(1),
        signature=[],
        nonce=rpc_felt(0),
        sender_address=pad_zero(invoke_content["contract_address"]),
        calldata=[pad_zero(hex(int(data))) for data in invoke_content["calldata"]],
    )

    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={"invoke_transaction": invoke_transaction},
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == {"transaction_hash"}
    assert receipt["transaction_hash"][:3] == "0x0"


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_declare_transaction_on_incorrect_contract(declare_content):
    """
    Add declare transaction on incorrect class
    """
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program="",
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class.get("abi", []),
    )

    declare_transaction = RpcBroadcastedDeclareTxn(
        type=declare_content["type"],
        max_fee=pad_zero(declare_content["max_fee"]),
        version=hex(SUPPORTED_TX_VERSION),
        signature=[pad_zero(sig) for sig in declare_content["signature"]],
        nonce=pad_zero(declare_content["nonce"]),
        contract_class=rpc_contract_class,
        sender_address=pad_zero(declare_content["sender_address"]),
    )

    ex = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )

    assert ex["error"] == {"code": 50, "message": "Invalid contract class"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_declare_transaction(declare_content):
    """
    Add declare transaction
    """
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program=declare_content["contract_class"]["program"],
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class.get("abi", []),
    )

    declare_transaction = RpcBroadcastedDeclareTxn(
        type=declare_content["type"],
        max_fee=pad_zero(declare_content["max_fee"]),
        version=hex(SUPPORTED_TX_VERSION),
        signature=[pad_zero(sig) for sig in declare_content["signature"]],
        nonce=pad_zero(declare_content["nonce"]),
        contract_class=rpc_contract_class,
        sender_address=pad_zero(declare_content["sender_address"]),
    )

    resp = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == set(["transaction_hash", "class_hash"])
    assert receipt["transaction_hash"][:3] == "0x0"
    assert receipt["class_hash"][:3] == "0x0"


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_deploy_transaction_on_incorrect_contract(deploy_content):
    """
    Add deploy transaction on incorrect class
    """
    contract_definition = deploy_content["contract_definition"]
    salt = pad_zero(deploy_content["contract_address_salt"])
    calldata = [pad_zero(data) for data in deploy_content["constructor_calldata"]]
    pad_zero_entry_points(contract_definition["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program="",
        entry_points_by_type=contract_definition["entry_points_by_type"],
        abi=contract_definition.get("abi", []),
    )

    deploy_transaction = RpcBroadcastedDeployTxn(
        contract_class=rpc_contract_class,
        version=hex(SUPPORTED_TX_VERSION),
        type=deploy_content["type"],
        contract_address_salt=salt,
        constructor_calldata=calldata,
    )

    ex = rpc_call(
        "starknet_addDeployTransaction",
        params={"deploy_transaction": deploy_transaction},
    )

    assert ex["error"] == {"code": 50, "message": "Invalid contract class"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_deploy_transaction(deploy_content):
    """
    Add deploy transaction
    """
    contract_definition = deploy_content["contract_definition"]
    salt = pad_zero(deploy_content["contract_address_salt"])
    calldata = [pad_zero(data) for data in deploy_content["constructor_calldata"]]
    pad_zero_entry_points(contract_definition["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program=deploy_content["contract_definition"]["program"],
        entry_points_by_type=contract_definition["entry_points_by_type"],
        abi=contract_definition.get("abi", []),
    )

    deploy_transaction = RpcBroadcastedDeployTxn(
        contract_class=rpc_contract_class,
        version=hex(SUPPORTED_TX_VERSION),
        type=deploy_content["type"],
        contract_address_salt=salt,
        constructor_calldata=calldata,
    )

    resp = rpc_call(
        "starknet_addDeployTransaction",
        params={"deploy_transaction": deploy_transaction},
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == set(["transaction_hash", "contract_address"])

    assert receipt["transaction_hash"][:3] == "0x0"
    assert receipt["contract_address"][:3] == "0x0"
