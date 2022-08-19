"""
Tests RPC rpc_call
"""

import pytest
from starkware.starknet.public.abi import get_selector_from_name

from .rpc_utils import rpc_call, pad_zero


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call(deploy_info):
    """
    Call contract
    """
    contract_address: str = deploy_info["address"]

    resp = rpc_call(
        "starknet_call", params={
            "request": {
                "contract_address": pad_zero(contract_address),
                "entry_point_selector": hex(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest"
        }
    )
    result = resp["result"]

    assert result["result"] == ["0x00"]


@pytest.mark.usefixtures("run_devnet_in_background", "deploy_info")
def test_call_raises_on_incorrect_contract_address():
    """
    Call contract with incorrect address
    """
    ex = rpc_call(
        "starknet_call", params={
            "request": {
                "contract_address": "0x07b529269b82f3f3ebbb2c463a9e1edaa2c6eea8fa308ff70b30398766a2e20c",
                "entry_point_selector": hex(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "latest"
        }
    )

    assert ex["error"] == {
        "code": 20,
        "message": "Contract not found"
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_raises_on_incorrect_selector(deploy_info):
    """
    Call contract with incorrect entry point selector
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call", params={
            "request": {
                "contract_address": pad_zero(contract_address),
                "entry_point_selector": hex(get_selector_from_name("xxxxxxx")),
                "calldata": [],
            },
            "block_id": "latest"
        }
    )

    assert ex["error"] == {
        "code": 21,
        "message": "Invalid message selector"
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_raises_on_invalid_calldata(deploy_info):
    """
    Call contract with incorrect calldata
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call", params={
            "request": {
                "contract_address": pad_zero(contract_address),
                "entry_point_selector": hex(get_selector_from_name("get_balance")),
                "calldata": ["a", "b", "123"],
            },
            "block_id": "latest"
        }
    )

    assert ex["error"] == {
        "code": 22,
        "message": "Invalid call data"
    }


@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_raises_on_incorrect_block_hash(deploy_info):
    """
    Call contract with incorrect block hash
    """
    contract_address: str = deploy_info["address"]

    ex = rpc_call(
        "starknet_call", params={
            "request": {
                "contract_address": pad_zero(contract_address),
                "entry_point_selector": hex(get_selector_from_name("get_balance")),
                "calldata": [],
            },
            "block_id": "0x0"
        }
    )

    assert ex["error"] == {
        "code": -1,
        "message": "Calls with block_id != 'latest' are not supported currently."
    }
