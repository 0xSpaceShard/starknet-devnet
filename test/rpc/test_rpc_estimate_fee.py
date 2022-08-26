"""
Tests RPC estimate fee
"""

from __future__ import annotations

from test.rpc.rpc_utils import rpc_call_background_devnet
from test.shared import CONTRACT_PATH
from test.util import deploy

import pytest

from starkware.starknet.definitions import constants
from starkware.starknet.public.abi import get_selector_from_name

from starknet_devnet.blueprints.rpc.structures.payloads import RpcInvokeTransaction
from starknet_devnet.constants import DEFAULT_GAS_PRICE


def common_estimate_response(response):
    """Expected response from RPC estimate_fee request"""
    result = response["result"]
    gas_price: int = int(result["gas_price"], 16)
    overall_fee: int = int(result["overall_fee"], 16)
    gas_consumed: int = int(result["gas_consumed"], 16)

    assert gas_price == DEFAULT_GAS_PRICE
    assert overall_fee == gas_consumed * gas_price


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background", [["--gas-price", str(DEFAULT_GAS_PRICE)]], indirect=True
)
def test_estimate_happy_path():
    """Happy path estimate_fee call"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    txn: RpcInvokeTransaction = {
        "contract_address": deploy_info["address"],
        "entry_point_selector": hex(get_selector_from_name("sum_point_array")),
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        # It is not verified and might be removed in next RPC version
        "transaction_hash": "0x00",
        "max_fee": "0x00",
        "version": hex(constants.TRANSACTION_VERSION),
        "signature": [],
        "nonce": "0x00",
        "type": "INVOKE",
    }
    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_with_genesis_block(rpc_invoke_tx_common):
    """Call without transaction, expect pass with gas_price zero"""
    txn: RpcInvokeTransaction = {
        "contract_address": "0x62230ea046a9a5fbc261ac77d03c8d41e5d442db2284587570ab46455fd2488",
        "entry_point_selector": "0x2f0b3c5710379609eb5495f1ecd348cb28167711b73609fe565a72734550354",
        "calldata": ["0x0a", "0x014", "0x00"],
        **rpc_invoke_tx_common,
    }
    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    common_estimate_response(response)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_with_invalid_call_data(rpc_invoke_tx_common):
    """Call estimate fee with invalid data on body"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    txn: RpcInvokeTransaction = {
        "contract_address": deploy_info["address"],
        "entry_point_selector": hex(get_selector_from_name("sum_point_array")),
        "calldata": ["10", "20"],
        **rpc_invoke_tx_common,
    }
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 22, "message": "Invalid call data"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_with_invalid_contract_address(rpc_invoke_tx_common):
    """Call estimate fee with invalid data on body"""
    txn: RpcInvokeTransaction = {
        "contract_address": "0x01",
        "entry_point_selector": hex(get_selector_from_name("sum_point_array")),
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        **rpc_invoke_tx_common,
    }
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 20, "message": "Contract not found"}


@pytest.mark.usefixtures("run_devnet_in_background")
def test_estimate_fee_with_invalid_message_selector(rpc_invoke_tx_common):
    """Call estimate fee with invalid data on body"""
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    txn: RpcInvokeTransaction = {
        "contract_address": deploy_info["address"],
        "entry_point_selector": "0x01",
        "calldata": ["0x02", "0x01", "0x02", "0x03", "0x04"],
        **rpc_invoke_tx_common,
    }
    ex = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    assert ex["error"] == {"code": 21, "message": "Invalid message selector"}


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background", [["--gas-price", str(DEFAULT_GAS_PRICE)]], indirect=True
)
def test_estimate_fee_with_complete_request_data(rpc_invoke_tx_common):
    """Estimate fee with complete request data"""

    deploy_info = deploy(CONTRACT_PATH, ["0"])

    txn: RpcInvokeTransaction = {
        "contract_address": deploy_info["address"],
        "entry_point_selector": "0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320",
        "calldata": ["0x0a", "0x014"],
        **rpc_invoke_tx_common,
    }
    response = rpc_call_background_devnet(
        "starknet_estimateFee", {"request": txn, "block_id": "latest"}
    )

    common_estimate_response(response)
