"""
Tests RPC estimate fee
"""

from __future__ import annotations

from test.account import _get_signature, get_nonce
from test.rpc.rpc_utils import rpc_call_background_devnet
from test.rpc.test_rpc_transactions import pad_zero_entry_points
from test.shared import (
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    SUPPORTED_RPC_TX_VERSION,
)
from test.test_account import deploy_empty_contract

import pytest
from starkware.starknet.core.os.transaction_hash.transaction_hash import (
    calculate_declare_transaction_hash,
)
from starkware.starknet.definitions.general_config import StarknetChainId
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import (
    DEFAULT_DECLARE_SENDER_ADDRESS,
)
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program

from starknet_devnet.account_util import get_execute_args
from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcBroadcastedDeclareTxn,
    RpcBroadcastedInvokeTxnV0,
    RpcBroadcastedInvokeTxnV1,
    RpcContractClass,
    RpcInvokeTransactionV0,
)
from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.constants import DEFAULT_GAS_PRICE, LEGACY_RPC_TX_VERSION


def common_estimate_response(response):
    """Expected response from RPC estimate_fee request"""
    result = response["result"]
    gas_price: int = int(result["gas_price"], 16)
    overall_fee: int = int(result["overall_fee"], 16)
    gas_consumed: int = int(result["gas_consumed"], 16)

    assert gas_price == DEFAULT_GAS_PRICE
    assert overall_fee == gas_consumed * gas_price


def get_predeployed_acc_execute_args(calls):
    """Get execute arguments with predeployed account"""
    return get_execute_args(
        calls=calls,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        nonce=0,
        version=SUPPORTED_RPC_TX_VERSION,
        max_fee=0,
    )


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_happy_path_v0():
    """Happy path estimate_fee call with tx v0"""
    contract_address = deploy_empty_contract()["address"]

    txn: RpcBroadcastedInvokeTxnV0 = {
        "contract_address": contract_address,
        "entry_point_selector": hex(get_selector_from_name("sum_point_array")),
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        "max_fee": rpc_felt(0),
        "version": hex(LEGACY_RPC_TX_VERSION),
        "signature": [],
        "type": "INVOKE",
    }
    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_happy_path():
    """Happy path estimate_fee call"""
    contract_address = deploy_empty_contract()["address"]

    calls = [(contract_address, "sum_point_array", [2, 10, 20, 30, 40])]
    signature, execute_calldata = get_predeployed_acc_execute_args(calls)

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(0),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata],
    )

    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": invoke_transaction, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_declare_v0(declare_content):
    """Test estimate_fee with declare transaction"""
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program=contract_class["program"],
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class["abi"],
    )

    declare_transaction = RpcBroadcastedDeclareTxn(
        type=declare_content["type"],
        max_fee=rpc_felt(declare_content["max_fee"]),
        version=hex(LEGACY_RPC_TX_VERSION),
        signature=[],
        nonce=rpc_felt(0),
        contract_class=rpc_contract_class,
        sender_address=rpc_felt(DEFAULT_DECLARE_SENDER_ADDRESS),
    )

    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": declare_transaction, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_fee_declare(declare_content):
    """Test estimate_fee with declare transaction"""
    contract_class = declare_content["contract_class"]
    pad_zero_entry_points(contract_class["entry_points_by_type"])

    rpc_contract_class = RpcContractClass(
        program=contract_class["program"],
        entry_points_by_type=contract_class["entry_points_by_type"],
        abi=contract_class["abi"],
    )

    contract_class = decompress_program({"contract_class": contract_class}, False)[
        "contract_class"
    ]
    contract_class = ContractClass.load(contract_class)

    nonce = get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)
    tx_hash = calculate_declare_transaction_hash(
        contract_class=contract_class,
        chain_id=StarknetChainId.TESTNET.value,
        sender_address=int(PREDEPLOYED_ACCOUNT_ADDRESS, 16),
        max_fee=0,
        nonce=nonce,
        version=SUPPORTED_RPC_TX_VERSION,
    )
    signature = _get_signature(tx_hash, PREDEPLOYED_ACCOUNT_PRIVATE_KEY)

    declare_transaction = RpcBroadcastedDeclareTxn(
        type=declare_content["type"],
        max_fee=rpc_felt(declare_content["max_fee"]),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(nonce),
        contract_class=rpc_contract_class,
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
    )

    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": declare_transaction, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_fee_with_invalid_call_data():
    """Call estimate fee with invalid data on body"""
    contract_address = deploy_empty_contract()["address"]

    calls = [(contract_address, "sum_point_array", [3, 10, 20, 30, 40])]
    signature, execute_calldata = get_predeployed_acc_execute_args(calls)

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(0),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata],
    )
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": invoke_transaction, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 22, "message": "Invalid call data"}


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_fee_with_invalid_contract_address():
    """Call estimate fee with invalid data on body"""
    txn: RpcInvokeTransactionV0 = {
        "contract_address": "0x01",
        "entry_point_selector": hex(get_selector_from_name("sum_point_array")),
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        "max_fee": rpc_felt(0),
        "version": hex(LEGACY_RPC_TX_VERSION),
        "signature": [],
        "type": "INVOKE",
    }
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 20, "message": "Contract not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_with_invalid_message_selector():
    """Call estimate fee with invalid data on body"""
    contract_address = deploy_empty_contract()["address"]

    txn: RpcInvokeTransactionV0 = {
        "contract_address": contract_address,
        "entry_point_selector": "0x01",
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        "max_fee": rpc_felt(0),
        "version": hex(LEGACY_RPC_TX_VERSION),
        "signature": [],
        "type": "INVOKE",
    }
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 21, "message": "Invalid message selector"}
