"""
Tests RPC transactions
"""

from __future__ import annotations

from typing import List

from starkware.starknet.definitions import constants
from starknet_devnet.blueprints.rpc import RpcContractClass

from .rpc_utils import rpc_call, get_block_with_transaction, pad_zero


def test_get_transaction_by_hash_deploy(deploy_info):
    """
    Get transaction by hash
    """
    transaction_hash: str = deploy_info["transaction_hash"]
    contract_address: str = deploy_info["address"]

    resp = rpc_call(
        "starknet_getTransactionByHash", params={"transaction_hash": transaction_hash}
    )
    transaction = resp["result"]

    assert transaction == {
        "txn_hash": pad_zero(transaction_hash),
        "contract_address": contract_address,
        "max_fee": "0x0",
        "calldata": [],
        "entry_point_selector": None,
        "signature": [],
        "version": "0x0"
    }


def test_get_transaction_by_hash_invoke(invoke_info):
    """
    Get transaction by hash
    """
    transaction_hash: str = invoke_info["transaction_hash"]
    contract_address: str = invoke_info["address"]
    entry_point_selector: str = invoke_info["entry_point_selector"]
    signature: List[str] = [pad_zero(hex(int(sig)))
                            for sig in invoke_info["signature"]]
    calldata: List[str] = [pad_zero(hex(int(data)))
                           for data in invoke_info["calldata"]]

    resp = rpc_call(
        "starknet_getTransactionByHash", params={"transaction_hash": transaction_hash}
    )
    transaction = resp["result"]

    assert transaction == {
        "txn_hash": pad_zero(transaction_hash),
        "contract_address": contract_address,
        "max_fee": "0x0",
        "calldata": calldata,
        "entry_point_selector": pad_zero(entry_point_selector),
        "signature": signature,
        "version": "0x0"
    }


def test_get_transaction_by_hash_declare(declare_info):
    """
    Get transaction by hash
    """
    transaction_hash: str = declare_info["transaction_hash"]
    signature: List[str] = [pad_zero(hex(int(sig)))
                            for sig in declare_info["signature"]]
    sender_address: str = declare_info["sender_address"]

    resp = rpc_call(
        "starknet_getTransactionByHash", params={"transaction_hash": transaction_hash}
    )
    transaction = resp["result"]

    assert transaction["txn_hash"] == pad_zero(transaction_hash)
    assert transaction["max_fee"] == "0x0"
    assert transaction["signature"] == signature
    assert transaction["version"] == "0x0"
    assert transaction["sender_address"] == pad_zero(sender_address)
    assert transaction["contract_class"]["entry_points_by_type"] == {
        "CONSTRUCTOR": [],
        "EXTERNAL": [
            {
                "offset": pad_zero("0x3a"),
                "selector": pad_zero("0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320")
            },
            {
                "offset": pad_zero("0x5b"),
                "selector": pad_zero("0x39e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695")
            }
        ],
        "L1_HANDLER": []
    }
    assert transaction["contract_class"]["program"] != ""


# pylint: disable=unused-argument
def test_get_transaction_by_hash_raises_on_incorrect_hash(deploy_info):
    """
    Get transaction by incorrect hash
    """
    ex = rpc_call(
        "starknet_getTransactionByHash", params={"transaction_hash": "0x0"}
    )

    assert ex["error"] == {
        "code": 25,
        "message": "Invalid transaction hash"
    }


def test_get_transaction_by_block_hash_and_index(deploy_info):
    """
    Get transaction by block hash and transaction index
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    transaction_hash: str = deploy_info["transaction_hash"]
    contract_address: str = deploy_info["address"]
    block_hash: str = block["block_hash"]
    index: int = 0

    resp = rpc_call(
        "starknet_getTransactionByBlockHashAndIndex", params={
            "block_hash": block_hash,
            "index": index
        }
    )
    transaction = resp["result"]

    assert transaction == {
        "txn_hash": pad_zero(transaction_hash),
        "contract_address": contract_address,
        "max_fee": "0x0",
        "calldata": [],
        "entry_point_selector": None,
        "signature": [],
        "version": "0x0"
    }


def test_get_transaction_by_block_hash_and_index_raises_on_incorrect_block_hash(deploy_info):
    """
    Get transaction by incorrect block hash
    """
    ex = rpc_call(
        "starknet_getTransactionByBlockHashAndIndex", params={
            "block_hash": "0x0",
            "index": 0
        }
    )

    assert ex["error"] == {
        "code": 24,
        "message": "Invalid block hash"
    }


def test_get_transaction_by_block_hash_and_index_raises_on_incorrect_index(deploy_info):
    """
    Get transaction by block hash and incorrect transaction index
    """
    block = get_block_with_transaction(deploy_info["transaction_hash"])
    block_hash: str = block["block_hash"]

    ex = rpc_call(
        "starknet_getTransactionByBlockHashAndIndex", params={
            "block_hash": block_hash,
            "index": 999999
        }
    )

    assert ex["error"] == {
        "code": 27,
        "message": "Invalid transaction index in a block"
    }


def test_get_transaction_by_block_number_and_index(deploy_info):
    """
    Get transaction by block number and transaction index
    """
    transaction_hash: str = deploy_info["transaction_hash"]
    contract_address: str = deploy_info["address"]
    block = get_block_with_transaction(transaction_hash)
    block_number: int = block["block_number"]
    index: int = 0

    resp = rpc_call(
        "starknet_getTransactionByBlockNumberAndIndex", params={
            "block_number": block_number,
            "index": index
        }
    )
    transaction = resp["result"]

    assert transaction == {
        "txn_hash": pad_zero(transaction_hash),
        "contract_address": contract_address,
        "max_fee": "0x0",
        "calldata": [],
        "entry_point_selector": None,
        "signature": [],
        "version": "0x0"
    }


def test_get_transaction_by_block_number_and_index_raises_on_incorrect_block_number(deploy_info):
    """
    Get transaction by incorrect block number
    """
    ex = rpc_call(
        "starknet_getTransactionByBlockNumberAndIndex", params={
            "block_number": 99999,
            "index": 0
        }
    )

    assert ex["error"] == {
        "code": 26,
        "message": "Invalid block number"
    }


def test_get_transaction_by_block_number_and_index_raises_on_incorrect_index(deploy_info):
    """
    Get transaction by block hash and incorrect transaction index
    """
    block_number: int = 0

    ex = rpc_call(
        "starknet_getTransactionByBlockNumberAndIndex", params={
            "block_number": block_number,
            "index": 99999
        }
    )

    assert ex["error"] == {
        "code": 27,
        "message": "Invalid transaction index in a block"
    }


def test_get_declare_transaction_receipt(declare_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = declare_info["transaction_hash"]

    resp = rpc_call(
        "starknet_getTransactionReceipt", params={
            "transaction_hash": transaction_hash
        }
    )
    receipt = resp["result"]

    assert receipt == {
        "txn_hash": pad_zero(transaction_hash),
        "status": "ACCEPTED_ON_L2",
        "statusData": None,
        "actual_fee": "0x0"
    }


def test_get_invoke_transaction_receipt(invoke_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = invoke_info["transaction_hash"]

    resp = rpc_call(
        "starknet_getTransactionReceipt", params={
            "transaction_hash": transaction_hash
        }
    )
    receipt = resp["result"]

    # Standard == receipt dict test cannot be done here, because invoke transaction fails since no contracts
    # are actually deployed on devnet, when running test without @devnet_in_background
    assert receipt["txn_hash"] == pad_zero(transaction_hash)
    assert receipt["actual_fee"] == "0x0"
    assert receipt["l1_origin_message"] is None
    assert receipt["events"] == []
    assert receipt["messages_sent"] == []


def test_get_transaction_receipt_on_incorrect_hash(deploy_info):
    """
    Get transaction receipt by incorrect hash
    """
    ex = rpc_call(
        "starknet_getTransactionReceipt", params={
            "transaction_hash": "0x0"
        }
    )

    assert ex["error"] == {
        "code": 25,
        "message": "Invalid transaction hash"
    }


def test_get_deploy_transaction_receipt(deploy_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = deploy_info["transaction_hash"]

    resp = rpc_call(
        "starknet_getTransactionReceipt", params={
            "transaction_hash": transaction_hash
        }
    )
    receipt = resp["result"]

    assert receipt == {
        "txn_hash": pad_zero(transaction_hash),
        "status": "ACCEPTED_ON_L2",
        "statusData": None,
        "actual_fee": "0x0"
    }


def test_add_invoke_transaction(invoke_content):
    """
    Add invoke transaction
    """
    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={
            "function_invocation": {
                "contract_address": invoke_content["contract_address"],
                "entry_point_selector": invoke_content["entry_point_selector"],
                "calldata": invoke_content["calldata"],
            },
            "signature": invoke_content["signature"],
            "max_fee": hex(0),
            "version": hex(constants.TRANSACTION_VERSION),
        }
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == {"transaction_hash"}
    assert receipt["transaction_hash"][:3] == "0x0"


def test_add_declare_transaction_on_incorrect_contract(declare_content):
    """
    Add declare transaction on incorrect class
    """
    contract_class = declare_content["contract_class"]

    rpc_contract = RpcContractClass(
        program="",
        entry_points_by_type=contract_class["entry_points_by_type"],
    )

    ex = rpc_call(
        "starknet_addDeclareTransaction",
        params={
            "contract_class": rpc_contract,
            "version": hex(constants.TRANSACTION_VERSION),
        }
    )

    assert ex["error"] == {
        "code": 50,
        "message": "Invalid contract class"
    }


def test_add_declare_transaction(declare_content):
    """
    Add declare transaction
    """
    contract_class = declare_content["contract_class"]

    rpc_contract = RpcContractClass(
        program=contract_class["program"],
        entry_points_by_type=contract_class["entry_points_by_type"],
    )

    resp = rpc_call(
        "starknet_addDeclareTransaction",
        params={
            "contract_class": rpc_contract,
            "version": hex(constants.TRANSACTION_VERSION),
        }
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == set(["transaction_hash", "class_hash"])
    assert receipt["transaction_hash"][:3] == "0x0"
    assert receipt["class_hash"][:3] == "0x0"


def test_add_deploy_transaction_on_incorrect_contract(deploy_content):
    """
    Add deploy transaction on incorrect class
    """
    contract_definition = deploy_content["contract_definition"]
    salt = deploy_content["contract_address_salt"]
    calldata = [hex(data) for data in deploy_content["constructor_calldata"]]

    rpc_contract = RpcContractClass(
        program="",
        entry_points_by_type=contract_definition["entry_points_by_type"],
    )

    ex = rpc_call(
        "starknet_addDeployTransaction",
        params={
            "contract_address_salt": salt,
            "constructor_calldata": calldata,
            "contract_definition": rpc_contract,
        }
    )

    assert ex["error"] == {
        "code": 50,
        "message": "Invalid contract class"
    }


def test_add_deploy_transaction(deploy_content):
    """
    Add deploy transaction
    """
    contract_definition = deploy_content["contract_definition"]
    salt = deploy_content["contract_address_salt"]
    calldata = [hex(data) for data in deploy_content["constructor_calldata"]]

    rpc_contract = RpcContractClass(
        program=contract_definition["program"],
        entry_points_by_type=contract_definition["entry_points_by_type"],
    )

    resp = rpc_call(
        "starknet_addDeployTransaction",
        params={
            "contract_address_salt": salt,
            "constructor_calldata": calldata,
            "contract_definition": rpc_contract,
        }
    )
    receipt = resp["result"]

    assert set(receipt.keys()) == set(["transaction_hash", "contract_address"])

    assert receipt["transaction_hash"][:3] == "0x0"
    assert receipt["contract_address"][:3] == "0x0"
