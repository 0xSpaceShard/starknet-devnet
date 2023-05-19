"""
Tests RPC transactions
"""

from __future__ import annotations

from test.account import (
    _get_signature,
    declare,
    declare_and_deploy_with_chargeable,
    get_nonce,
)
from test.rpc.conftest import prepare_deploy_account_tx, rpc_deploy_account_from_gateway
from test.rpc.rpc_utils import (
    deploy_and_invoke_storage_contract,
    gateway_call,
    get_block_with_transaction,
    is_felt,
    rpc_call,
)
from test.shared import (
    ABI_PATH,
    CONTRACT_PATH,
    DEPRECATED_RPC_DECLARE_TX_VERSION,
    EXPECTED_UDC_ADDRESS,
    INCORRECT_GENESIS_BLOCK_HASH,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    STARKNET_CLI_ACCOUNT_ABI_PATH,
    SUPPORTED_RPC_DECLARE_TX_VERSION,
    SUPPORTED_RPC_TX_VERSION,
)
from test.test_declare_v2 import load_cairo1_contract
from test.util import assert_tx_status, call, load_contract_class, mint, send_tx
from typing import List

import pytest
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
    calculate_deprecated_declare_transaction_hash,
)
from starkware.starknet.definitions.general_config import (
    DEFAULT_CHAIN_ID,
    StarknetChainId,
)
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.public.abi import (
    get_selector_from_name,
    get_storage_var_address,
)
from starkware.starknet.services.api.gateway.transaction_utils import compress_program
from starkware.starknet.wallets.open_zeppelin import sign_invoke_tx

from starknet_devnet.account_util import get_execute_args
from starknet_devnet.blueprints.rpc.structures.payloads import (
    DeprecatedEntryPoints,
    RpcBroadcastedDeclareTxnV1,
    RpcBroadcastedDeclareTxnV2,
    RpcBroadcastedInvokeTxnV0,
    RpcBroadcastedInvokeTxnV1,
    RpcDeprecatedContractClass,
    rpc_contract_class,
)
from starknet_devnet.blueprints.rpc.structures.types import Signature, rpc_txn_type
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.constants import LEGACY_RPC_TX_VERSION


def pad_zero_entry_points(entry_points: DeprecatedEntryPoints) -> None:
    """
    Pad zero every selector in entry points in contract_class
    """

    def pad_selector(entry_point):
        return {
            "offset": rpc_felt(entry_point["offset"]),
            "selector": rpc_felt(entry_point["selector"]),
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
    block = get_block_with_transaction(deploy_info["tx_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = deploy_info["tx_hash"]

    signature: Signature = [rpc_felt(sig) for sig in block_tx["signature"]]
    calldata: List[str] = [rpc_felt(data) for data in block_tx["calldata"]]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": rpc_felt(transaction_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": rpc_felt(transaction_hash),
        "version": hex(SUPPORTED_RPC_TX_VERSION),
        "type": rpc_txn_type(block_tx["type"]),
        "calldata": calldata,
        "max_fee": rpc_felt(block_tx["max_fee"]),
        "nonce": rpc_felt(block_tx["nonce"]),
        "signature": signature,
        "sender_address": rpc_felt(block_tx["sender_address"]),
    }


@pytest.mark.usefixtures("devnet_with_account")
def test_get_transaction_by_hash_invoke():
    """
    Get transaction by hash
    """
    _, invoke_tx_hash = deploy_and_invoke_storage_contract(value=30)

    block = get_block_with_transaction(invoke_tx_hash)
    block_tx = block["transactions"][0]
    signature: Signature = [rpc_felt(sig) for sig in block_tx["signature"]]
    calldata: List[str] = [rpc_felt(data) for data in block_tx["calldata"]]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": rpc_felt(invoke_tx_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": rpc_felt(invoke_tx_hash),
        "max_fee": rpc_felt(block_tx["max_fee"]),
        "version": hex(SUPPORTED_RPC_TX_VERSION),
        "signature": signature,
        "nonce": rpc_felt(0),
        "type": rpc_txn_type(block_tx["type"]),
        "sender_address": rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        "calldata": calldata,
    }


@pytest.mark.usefixtures("devnet_with_account")
def test_get_transaction_by_hash_declare():
    """
    Get transaction by hash
    """
    max_fee = int(4e16)
    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=max_fee,
    )

    block = get_block_with_transaction(declare_info["tx_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = declare_info["tx_hash"]
    signature: Signature = [rpc_felt(sig) for sig in block_tx["signature"]]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": rpc_felt(transaction_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "transaction_hash": rpc_felt(transaction_hash),
        "max_fee": rpc_felt(block_tx["max_fee"]),
        "version": hex(SUPPORTED_RPC_TX_VERSION),
        "signature": signature,
        "nonce": rpc_felt(0),
        "type": rpc_txn_type(block_tx["type"]),
        "sender_address": rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        "class_hash": rpc_felt(block_tx["class_hash"]),
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_hash_deploy_account(deploy_account_info):
    """
    Get transaction by hash
    """
    tx_hash = deploy_account_info["transaction_hash"]

    block = get_block_with_transaction(tx_hash)
    block_tx = block["transactions"][0]

    resp = rpc_call(
        "starknet_getTransactionByHash",
        params={"transaction_hash": rpc_felt(tx_hash)},
    )
    transaction = resp["result"]

    assert transaction == {
        "contract_address_salt": rpc_felt(block_tx["contract_address_salt"]),
        "constructor_calldata": [
            rpc_felt(data) for data in block_tx["constructor_calldata"]
        ],
        "class_hash": rpc_felt(block_tx["class_hash"]),
        "type": rpc_txn_type(block_tx["type"]),
        "max_fee": rpc_felt(block_tx["max_fee"]),
        "version": hex(SUPPORTED_RPC_TX_VERSION),
        "signature": [rpc_felt(sig) for sig in block_tx["signature"]],
        "nonce": rpc_felt(block_tx["nonce"]),
        "transaction_hash": rpc_felt(tx_hash),
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_hash_raises_on_incorrect_hash():
    """
    Get transaction by incorrect hash
    """
    ex = rpc_call("starknet_getTransactionByHash", params={"transaction_hash": "0x0"})

    assert ex["error"] == {"code": 25, "message": "Transaction hash not found"}


@pytest.mark.usefixtures("devnet_with_account")
def test_get_transaction_by_block_id_and_index(deploy_info):
    """
    Get transaction by block id and transaction index
    """
    block = get_block_with_transaction(deploy_info["tx_hash"])
    block_tx = block["transactions"][0]
    transaction_hash: str = deploy_info["tx_hash"]
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

    signature: Signature = [rpc_felt(sig) for sig in block_tx["signature"]]
    calldata: List[str] = [rpc_felt(data) for data in block_tx["calldata"]]

    assert transaction == {
        "calldata": calldata,
        "signature": signature,
        "sender_address": rpc_felt(block_tx["sender_address"]),
        "transaction_hash": rpc_felt(transaction_hash),
        "max_fee": rpc_felt(block_tx["max_fee"]),
        "nonce": rpc_felt(block_tx["nonce"]),
        "type": rpc_txn_type(block_tx["type"]),
        "version": hex(SUPPORTED_RPC_TX_VERSION),
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_block_id_and_index_raises_on_incorrect_block_hash():
    """
    Get transaction by incorrect block id
    """
    ex = rpc_call(
        "starknet_getTransactionByBlockIdAndIndex",
        params={
            "block_id": {"block_hash": rpc_felt(INCORRECT_GENESIS_BLOCK_HASH)},
            "index": 0,
        },
    )

    assert ex["error"] == {"code": 24, "message": "Block not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_transaction_by_block_id_and_index_raises_on_incorrect_index(deploy_info):
    """
    Get transaction by block hash and incorrect transaction index
    """
    block = get_block_with_transaction(deploy_info["tx_hash"])
    block_hash: str = block["block_hash"]

    ex = rpc_call(
        "starknet_getTransactionByBlockIdAndIndex",
        params={
            "block_id": {
                "block_hash": rpc_felt(block_hash),
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
        params={"transaction_hash": rpc_felt(transaction_hash)},
    )
    receipt = resp["result"]

    assert receipt == {
        "transaction_hash": rpc_felt(transaction_hash),
        "actual_fee": rpc_felt(0),
        "status": "ACCEPTED_ON_L2",
        "block_hash": rpc_felt(block["block_hash"]),
        "block_number": block["block_number"],
        "type": "DECLARE",
        "messages_sent": [],
        "events": [],
    }


@pytest.mark.usefixtures("devnet_with_account")
def test_get_invoke_transaction_receipt():
    """
    Get transaction receipt
    """

    _, invoke_tx_hash = deploy_and_invoke_storage_contract(value=30)

    gateway_receipt = gateway_call(
        "get_transaction_receipt", transactionHash=invoke_tx_hash
    )
    event = gateway_receipt["events"][0]

    block = get_block_with_transaction(invoke_tx_hash)
    block_tx = block["transactions"][0]

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": rpc_felt(invoke_tx_hash)},
    )
    receipt = resp["result"]

    assert receipt == {
        "transaction_hash": rpc_felt(invoke_tx_hash),
        "actual_fee": rpc_felt(block_tx["max_fee"]),
        "status": "ACCEPTED_ON_L2",
        "block_hash": rpc_felt(block["block_hash"]),
        "block_number": block["block_number"],
        "type": rpc_txn_type(block_tx["type"]),
        "messages_sent": [],
        "events": [
            {
                "from_address": rpc_felt(event["from_address"]),
                "data": [rpc_felt(data) for data in event["data"]],
                "keys": [rpc_felt(key) for key in event["keys"]],
            }
        ],
    }


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
    transaction_hash: str = deploy_info["tx_hash"]
    block = get_block_with_transaction(transaction_hash)

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": rpc_felt(transaction_hash)},
    )
    receipt = resp["result"]

    assert len(block["transaction_receipts"]) == 1
    block_receipt = block["transaction_receipts"][0]

    assert receipt == {
        "transaction_hash": rpc_felt(transaction_hash),
        "actual_fee": rpc_felt(block_receipt["actual_fee"]),
        "status": "ACCEPTED_ON_L2",
        "block_hash": rpc_felt(block["block_hash"]),
        "block_number": block["block_number"],
        "type": "INVOKE",
        "messages_sent": [],
        "events": [
            {
                "from_address": rpc_felt(event["from_address"]),
                "data": [rpc_felt(data) for data in event["data"]],
                "keys": [rpc_felt(key) for key in event["keys"]],
            }
            for event in block_receipt["events"]
        ],
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_deploy_account_transaction_receipt(deploy_account_info):
    """
    Get transaction receipt
    """
    transaction_hash: str = deploy_account_info["transaction_hash"]
    block = get_block_with_transaction(transaction_hash)
    gateway_receipt = block["transaction_receipts"][0]
    event = gateway_receipt["events"][0]

    resp = rpc_call(
        "starknet_getTransactionReceipt",
        params={"transaction_hash": rpc_felt(transaction_hash)},
    )
    receipt = resp["result"]

    assert receipt == {
        "contract_address": rpc_felt(deploy_account_info["address"]),
        "transaction_hash": rpc_felt(transaction_hash),
        "actual_fee": rpc_felt(gateway_receipt["actual_fee"]),
        "status": "ACCEPTED_ON_L2",
        "block_hash": rpc_felt(block["block_hash"]),
        "block_number": block["block_number"],
        "type": "DEPLOY_ACCOUNT",
        "messages_sent": [],
        "events": [
            {
                "from_address": rpc_felt(event["from_address"]),
                "data": [rpc_felt(data) for data in event["data"]],
                "keys": [rpc_felt(key) for key in event["keys"]],
            }
        ],
    }


@pytest.mark.usefixtures("devnet_with_account")
def test_add_invoke_transaction():
    """
    Add invoke transaction
    """
    initial_balance, amount1, amount2 = 100, 13, 56
    deploy_dict = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, [str(initial_balance)]
    )
    contract_address = deploy_dict["address"]
    max_fee = int(4e16)

    calls = [(contract_address, "increase_balance", [amount1, amount2])]
    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        nonce=0,
        version=SUPPORTED_RPC_TX_VERSION,
        max_fee=max_fee,
    )

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(max_fee),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata],
    )

    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={"invoke_transaction": invoke_transaction},
    )
    receipt = resp["result"]

    storage = gateway_call(
        "get_storage_at",
        contractAddress=contract_address,
        key=get_storage_var_address("balance"),
    )

    assert set(receipt.keys()) == {"transaction_hash"}
    assert is_felt(receipt["transaction_hash"])
    assert storage == hex(initial_balance + amount1 + amount2)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_invoke_transaction_v0():
    """
    Add invoke transaction with tx v0
    """
    initial_balance, amount1, amount2 = 100, 13, 56
    deploy_dict = declare_and_deploy_with_chargeable(
        CONTRACT_PATH, [str(initial_balance)]
    )
    contract_address = deploy_dict["address"]

    invoke_transaction = RpcBroadcastedInvokeTxnV0(
        type="INVOKE",
        max_fee=rpc_felt(0),
        version=hex(LEGACY_RPC_TX_VERSION),
        signature=[],
        nonce="0x0",
        contract_address=rpc_felt(contract_address),
        entry_point_selector=rpc_felt(get_selector_from_name("increase_balance")),
        calldata=[rpc_felt(amount1), rpc_felt(amount2)],
    )

    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={"invoke_transaction": invoke_transaction},
    )
    receipt = resp["result"]

    storage = gateway_call(
        "get_storage_at",
        contractAddress=contract_address,
        key=get_storage_var_address("balance"),
    )

    assert set(receipt.keys()) == {"transaction_hash"}
    assert is_felt(receipt["transaction_hash"])
    assert storage == hex(initial_balance + amount1 + amount2)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_declare_transaction_on_incorrect_contract(declare_content):
    """
    Add declare transaction on incorrect class
    """
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    contract_class = RpcDeprecatedContractClass(
        program="",
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class["abi"],
    )

    declare_transaction = RpcBroadcastedDeclareTxnV1(
        type=declare_content["type"],
        max_fee=rpc_felt(declare_content["max_fee"]),
        version=hex(DEPRECATED_RPC_DECLARE_TX_VERSION),
        signature=[rpc_felt(sig) for sig in declare_content["signature"]],
        nonce=rpc_felt(declare_content["nonce"]),
        contract_class=contract_class,
        sender_address=rpc_felt(declare_content["sender_address"]),
    )

    ex = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )

    assert ex["error"] == {"code": 50, "message": "Invalid contract class"}


@pytest.mark.usefixtures("devnet_with_account")
def test_add_declare_transaction_v2():
    """Add declare transaction v2"""
    contract_class, _, compiled_class_hash = load_cairo1_contract()

    max_fee = int(4e16)
    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)

    tx_hash = calculate_declare_transaction_hash(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
        max_fee=max_fee,
        version=SUPPORTED_RPC_DECLARE_TX_VERSION,
        nonce=nonce,
    )

    signature = _get_signature(tx_hash, PREDEPLOYED_ACCOUNT_PRIVATE_KEY)

    declare_transaction = RpcBroadcastedDeclareTxnV2(
        contract_class=rpc_contract_class(contract_class),
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        compiled_class_hash=rpc_felt(compiled_class_hash),
        type="DECLARE",
        version=rpc_felt(SUPPORTED_RPC_DECLARE_TX_VERSION),
        nonce=rpc_felt(nonce),
        max_fee=rpc_felt(max_fee),
        signature=list(map(rpc_felt, signature)),
    )

    resp = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )

    receipt = resp["result"]

    assert set(receipt.keys()) == set(["transaction_hash", "class_hash"])
    assert is_felt(receipt["transaction_hash"])
    assert is_felt(receipt["class_hash"])


def _add_declare_transaction():
    contract_class = load_contract_class(CONTRACT_PATH)
    contract_class_dump = contract_class.dump()

    pad_zero_entry_points(contract_class_dump["entry_points_by_type"])

    _rpc_contract_class = RpcDeprecatedContractClass(
        program=compress_program(contract_class_dump["program"]),
        entry_points_by_type=contract_class_dump["entry_points_by_type"],
        abi=contract_class_dump["abi"],
    )
    max_fee = int(4e16)

    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)
    tx_hash = calculate_deprecated_declare_transaction_hash(
        contract_class=contract_class,
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
        max_fee=max_fee,
        nonce=nonce,
        version=SUPPORTED_RPC_TX_VERSION,
    )
    signature = _get_signature(tx_hash, PREDEPLOYED_ACCOUNT_PRIVATE_KEY)

    declare_transaction = RpcBroadcastedDeclareTxnV1(
        type="DECLARE",
        max_fee=rpc_felt(max_fee),
        version=hex(DEPRECATED_RPC_DECLARE_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(nonce),
        contract_class=_rpc_contract_class,
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
    )

    resp = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )
    return resp["result"]


@pytest.mark.usefixtures("devnet_with_account")
def test_add_declare_transaction_v1():
    """Add declare transaction"""
    receipt = _add_declare_transaction()

    assert set(receipt.keys()) == set(["transaction_hash", "class_hash"])
    assert is_felt(receipt["transaction_hash"])
    assert is_felt(receipt["class_hash"])
    assert_tx_status(receipt["transaction_hash"], "ACCEPTED_ON_L2")


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_declare_transaction_v0_fails(declare_content):
    """
    Adding declare transaction with tx v0 should fail
    """
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    contract_class = RpcDeprecatedContractClass(
        program=contract_class["program"],
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class["abi"],
    )

    declare_transaction = RpcBroadcastedDeclareTxnV1(
        type=declare_content["type"],
        max_fee=rpc_felt(declare_content["max_fee"]),
        version=hex(LEGACY_RPC_TX_VERSION),
        signature=[],
        nonce=rpc_felt(0),
        contract_class=contract_class,
        sender_address=rpc_felt(1),
    )

    ex = rpc_call(
        "starknet_addDeclareTransaction",
        params={"declare_transaction": declare_transaction},
    )

    assert ex["error"] == {"code": 50, "message": "Invalid contract class"}


@pytest.mark.usefixtures("devnet_with_account")
def test_deploy():
    """Deploy contract.cairo via UDC"""
    declaration_receipt = _add_declare_transaction()

    initial_balance = 10
    ctor_args = [initial_balance]

    calls = [
        (
            EXPECTED_UDC_ADDRESS,
            "deployContract",
            [
                int(declaration_receipt["class_hash"], 16),
                42,  # salt
                0,  # unique
                len(ctor_args),
                *ctor_args,
            ],
        )
    ]

    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)
    max_fee = int(4e16)
    signature, execute_calldata = get_execute_args(
        calls=calls,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        nonce=nonce,
        version=SUPPORTED_RPC_TX_VERSION,
        max_fee=max_fee,
    )

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(max_fee),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(nonce),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata],
    )

    resp = rpc_call(
        "starknet_addInvokeTransaction",
        params={"invoke_transaction": invoke_transaction},
    )
    deployment_receipt = resp["result"]
    assert_tx_status(deployment_receipt["transaction_hash"], "ACCEPTED_ON_L2")
    # to assert storage of the newly deployed contract, we would need to calculate it or extract it from UDC event


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_deploy_account_transaction_on_incorrect_class_hash(deploy_account_details):
    """
    Add deploy transaction on incorrect class
    """
    invalid_class_hash = 1337

    deploy_account_tx, address = prepare_deploy_account_tx(**deploy_account_details)
    rpc_deploy_account_tx = rpc_deploy_account_from_gateway(deploy_account_tx)
    rpc_deploy_account_tx["class_hash"] = rpc_felt(invalid_class_hash)

    mint(hex(address), amount=int(1e18))

    ex = rpc_call(
        "starknet_addDeployAccountTransaction",
        params={"deploy_account_transaction": rpc_deploy_account_tx},
    )
    assert ex["error"] == {"code": 28, "message": "Class hash not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_add_deploy_account_transaction(deploy_account_details):
    """Test the deployment of an account."""

    # the account class should already be declared

    # generate account with random keys and salt
    deploy_account_tx, address = prepare_deploy_account_tx(**deploy_account_details)
    rpc_deploy_account_tx = rpc_deploy_account_from_gateway(deploy_account_tx)

    tx_before = rpc_call(
        "starknet_addDeployAccountTransaction",
        params={"deploy_account_transaction": rpc_deploy_account_tx},
    )
    tx_before = tx_before["result"]

    # deployment should fail if no funds
    assert_tx_status(tx_before["transaction_hash"], "REJECTED")

    # fund the address of the account
    mint(hex(address), amount=int(1e18))

    # deploy the account
    tx_after = rpc_call(
        "starknet_addDeployAccountTransaction",
        params={"deploy_account_transaction": rpc_deploy_account_tx},
    )
    tx_after = tx_after["result"]
    assert_tx_status(tx_after["transaction_hash"], "ACCEPTED_ON_L2")

    # assert that contract can be interacted with
    retrieved_public_key = call(
        function="get_public_key",
        address=hex(address),
        abi_path=STARKNET_CLI_ACCOUNT_ABI_PATH,
    )
    assert int(retrieved_public_key, 16) == deploy_account_details["public_key"]

    # deploy a contract for testing
    init_balance = 10
    contract_deploy_info = declare_and_deploy_with_chargeable(
        contract=CONTRACT_PATH, inputs=[str(init_balance)]
    )
    contract_address = contract_deploy_info["address"]

    # increase balance of test contract
    invoke_tx = sign_invoke_tx(
        signer_address=address,
        private_key=deploy_account_details["private_key"],
        contract_address=int(contract_address, 16),
        selector=get_selector_from_name("increase_balance"),
        calldata=[10, 20],
        chain_id=DEFAULT_CHAIN_ID,
        max_fee=int(1e18),
        version=SUPPORTED_RPC_TX_VERSION,
        nonce=1,
    ).dump()

    invoke_tx = send_tx(invoke_tx, TransactionType.INVOKE_FUNCTION)
    assert_tx_status(invoke_tx["transaction_hash"], "ACCEPTED_ON_L2")

    # get balance of test contract
    balance_after = call(
        function="get_balance", address=contract_address, abi_path=ABI_PATH
    )
    assert balance_after == "40"
