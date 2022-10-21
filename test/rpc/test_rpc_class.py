"""
Tests RPC contract class
"""
from test.rpc.rpc_utils import rpc_call

import pytest
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program

from starknet_devnet.blueprints.rpc.utils import BlockId, rpc_felt

EXPECTED_ENTRY_POINTS = {
    "CONSTRUCTOR": [
        {
            "offset": "0x35",
            "selector": "0x028ffe4ff0f226a9107253e17a904099aa4f63a02a5621de0576e5aa71bc5194",
        }
    ],
    "EXTERNAL": [
        {
            "offset": "0x54",
            "selector": "0x0362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320",
        },
        {
            "offset": "0x76",
            "selector": "0x039e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695",
        },
        {
            "offset": "0xa1",
            "selector": "0x03cd0a5d52a5573221431e4a61c6bdffd7f157bd278be73f332f0b10f12d895b",
        },
    ],
    "L1_HANDLER": [],
}

# abi from ABI_PATH (test/shared.py), but with all stateMutability omitted and constructor mapped to function
EXPECTED_ABI = [
    {
        "type": "struct",
        "name": "Point",
        "size": 2,
        "members": [
            {"name": "x", "offset": 0, "type": "felt"},
            {"name": "y", "offset": 1, "type": "felt"},
        ],
    },
    {
        "type": "function",
        "name": "constructor",
        "inputs": [{"name": "initial_balance", "type": "felt"}],
        "outputs": [],
    },
    {
        "type": "function",
        "name": "increase_balance",
        "inputs": [
            {"name": "amount1", "type": "felt"},
            {"name": "amount2", "type": "felt"},
        ],
        "outputs": [],
    },
    {
        "type": "function",
        "name": "get_balance",
        "inputs": [],
        "outputs": [{"name": "res", "type": "felt"}],
    },
    {
        "type": "function",
        "name": "sum_point_array",
        "inputs": [
            {"name": "points_len", "type": "felt"},
            {"name": "points", "type": "Point*"},
        ],
        "outputs": [{"name": "res", "type": "Point"}],
    },
]


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_class(class_hash):
    """
    Test get contract class
    """
    resp = rpc_call(
        "starknet_getClass", params={"block_id": "latest", "class_hash": class_hash}
    )
    contract_class = resp["result"]

    assert contract_class["entry_points_by_type"] == EXPECTED_ENTRY_POINTS
    assert isinstance(contract_class["program"], str)
    decompress_program({"contract_class": contract_class}, False)
    assert contract_class["abi"] == EXPECTED_ABI


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_class_hash_at(deploy_info, class_hash):
    """
    Test get contract class at given hash
    """
    contract_address: str = deploy_info["address"]
    block_id: BlockId = "latest"

    resp = rpc_call(
        "starknet_getClassHashAt",
        params={"contract_address": rpc_felt(contract_address), "block_id": block_id},
    )
    rpc_class_hash = resp["result"]

    assert rpc_class_hash == class_hash


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_class_at(deploy_info):
    """
    Test get contract class at given contract address
    """
    contract_address: str = deploy_info["address"]
    block_id: BlockId = "latest"

    resp = rpc_call(
        "starknet_getClassAt",
        params={"contract_address": rpc_felt(contract_address), "block_id": block_id},
    )
    contract_class = resp["result"]

    assert contract_class["entry_points_by_type"] == EXPECTED_ENTRY_POINTS
    assert isinstance(contract_class["program"], str)
    decompress_program({"contract_class": contract_class}, False)
    assert contract_class["abi"] == EXPECTED_ABI
