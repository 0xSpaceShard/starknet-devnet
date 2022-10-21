"""
Tests RPC estimate fee
"""

from __future__ import annotations

from test.account import _get_execute_args, get_nonce
from test.rpc.rpc_utils import rpc_call_background_devnet
from test.shared import (
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    SUPPORTED_RPC_TX_VERSION,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from test.test_account import deploy_empty_contract

import pytest
from starkware.starknet.public.abi import get_selector_from_name

from starknet_devnet.blueprints.rpc.structures.payloads import (
    RpcInvokeTransactionV0,
    RpcBroadcastedInvokeTxnV0,
    RpcBroadcastedInvokeTxnV1,
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


def get_execute_args(calls):
    """Get execute arguments with predeployed account"""
    return _get_execute_args(
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
    signature, execute_calldata = get_execute_args(calls)

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
@pytest.mark.parametrize(
    "run_devnet_in_background",
    [["--gas-price", str(DEFAULT_GAS_PRICE), *PREDEPLOY_ACCOUNT_CLI_ARGS]],
    indirect=True,
)
def test_estimate_fee_with_invalid_call_data():
    """Call estimate fee with invalid data on body"""
    contract_address = deploy_empty_contract()["address"]

    calls = [(contract_address, "sum_point_array", [2, 10, 20, 30, 40])]
    signature, execute_calldata = get_execute_args(calls)

    invoke_transaction = RpcBroadcastedInvokeTxnV1(
        type="INVOKE",
        max_fee=rpc_felt(0),
        version=hex(SUPPORTED_RPC_TX_VERSION),
        signature=[rpc_felt(sig) for sig in signature],
        nonce=rpc_felt(get_nonce(PREDEPLOYED_ACCOUNT_ADDRESS)),
        sender_address=rpc_felt(PREDEPLOYED_ACCOUNT_ADDRESS),
        calldata=[rpc_felt(data) for data in execute_calldata][:-1],
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
