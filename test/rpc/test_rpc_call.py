"""
Tests RPC rpc_call
"""
from test.account import invoke
from test.rpc.rpc_utils import get_block_with_transaction, rpc_call
from test.shared import PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_PRIVATE_KEY

import pytest
from starkware.starknet.public.abi import get_selector_from_name

from starknet_devnet.blueprints.rpc.structures.types import PredefinedRpcErrorCode
from starknet_devnet.blueprints.rpc.utils import rpc_felt


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call(deploy_info, latest_block_id):
    """
    Call contract
    """
    contract_address: str = deploy_info["address"]

    resp = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": rpc_felt(contract_address),
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": latest_block_id,
        },
    )
    assert "error" not in resp
    result = resp["result"]

    assert result == ["0x45"]


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_call_raises_on_incorrect_contract_address():
    """
    Call contract with incorrect address
    """
    ex = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": "0x7b529269b82f3f3ebbb2c463a9e1edaa2c6eea8fa308ff70b30398766a2e20c",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest",
        },
    )

    assert ex["error"] == {"code": 20, "message": "Contract not found"}


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_call_raises_on_both_hash_and_number():
    """
    Call contract with both block hash and block number provided at the same time
    """
    ex = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": "0x7b529269b82f3f3ebbb2c463a9e1edaa2c6eea8fa308ff70b30398766a2e20c",
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": {"block_hash": "0x1234", "block_number": 1234},
        },
    )

    assert ex["error"] == {
        "code": -1,
        "message": "Parameters block_hash and block_number are mutually exclusive.",
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_raises_on_incorrect_selector(deploy_info):
    """
    Call contract with incorrect entry point selector
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": rpc_felt(contract_address),
                "entry_point_selector": rpc_felt(get_selector_from_name("xxxxxxx")),
                "calldata": [],
            },
            "block_id": "latest",
        },
    )

    assert ex["error"] == {"code": 21, "message": "Invalid message selector"}


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "calldata",
    [[123], ["1231", "wtf", "0x123"], [""]],
)
def test_call_raises_on_invalid_calldata(deploy_info, calldata):
    """
    Call contract with incorrect calldata
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": rpc_felt(contract_address),
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": calldata,
            },
            "block_id": "latest",
        },
    )

    assert ex["error"]["code"] == PredefinedRpcErrorCode.INVALID_PARAMS.value


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_raises_on_incorrect_block_hash(deploy_info):
    """
    Call contract with incorrect block hash
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call",
        params={
            "request": {
                "contract_address": rpc_felt(contract_address),
                "entry_point_selector": rpc_felt(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "0x0",
        },
    )

    assert ex["error"]["code"] == PredefinedRpcErrorCode.INVALID_PARAMS.value


@pytest.mark.usefixtures("devnet_with_account")
def test_call_on_old_block(deploy_info):
    """Correctly call contract on old state"""

    contract_address: str = deploy_info["address"]
    deployment_block = get_block_with_transaction(deploy_info["tx_hash"])

    invoke_tx_hash = invoke(
        calls=[(contract_address, "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    increment_block = get_block_with_transaction(invoke_tx_hash)
    assert increment_block["block_number"] == deployment_block["block_number"] + 1

    def call_and_assert(block_id: dict):
        resp = rpc_call(
            "starknet_call",
            params={
                "request": {
                    "contract_address": rpc_felt(contract_address),
                    "entry_point_selector": rpc_felt(
                        get_selector_from_name("get_balance")
                    ),
                    "calldata": [],
                },
                "block_id": block_id,
            },
        )

        assert "error" not in resp
        result = resp["result"]
        assert result == ["0x45"]

    call_and_assert({"block_number": deployment_block["block_number"]})
    call_and_assert({"block_hash": rpc_felt(deployment_block["block_hash"])})


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_with_invalid_method():
    """Call with an invalid method"""
    ex = rpc_call(method="obviously_invalid_method", params={})
    assert ex["error"] == {"code": -32601, "message": "Method not found"}
