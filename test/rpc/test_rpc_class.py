"""
Tests RPC contract class
"""
import json
from test.account import deploy, send_declare_v2
from test.rpc.rpc_utils import rpc_call
from test.shared import (
    ABI_1_PATH,
    ABI_PATH,
    CONTRACT_1_PATH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from test.test_declare_v2 import assert_declare_v2_accepted, load_cairo1_contract
from test.util import assert_tx_status, devnet_in_background

import pytest
from starkware.starknet.services.api.gateway.transaction_utils import decompress_program

from starknet_devnet.blueprints.rpc.utils import BlockId, rpc_felt

EXPECTED_ENTRY_POINTS = {
    "CONSTRUCTOR": [
        {
            "offset": "0x35",
            "selector": "0x28ffe4ff0f226a9107253e17a904099aa4f63a02a5621de0576e5aa71bc5194",
        }
    ],
    "EXTERNAL": [
        {
            "offset": "0x54",
            "selector": "0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320",
        },
        {
            "offset": "0x76",
            "selector": "0x39e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695",
        },
        {
            "offset": "0xa1",
            "selector": "0x3cd0a5d52a5573221431e4a61c6bdffd7f157bd278be73f332f0b10f12d895b",
        },
    ],
    "L1_HANDLER": [],
}


def assert_correct_cairo_1_contract(contract_class):
    """
    Assert that the provided cairo 1 contract is equal to the expected one.
    """
    with open(CONTRACT_1_PATH, mode="r", encoding="utf-8") as expected_contract, open(
        ABI_1_PATH, mode="r", encoding="utf-8"
    ) as expected_abi:
        expected_contract_json = json.loads(expected_contract.read())
        expected_abi_json = json.loads(expected_abi.read())

    assert (
        contract_class["entry_points_by_type"]
        == expected_contract_json["entry_points_by_type"]
    )
    assert contract_class["abi"] == json.dumps(expected_abi_json)


@pytest.mark.usefixtures("run_devnet_in_background")
def test_get_deprecated_class(class_hash):
    """
    Test get contract class
    """
    with open(ABI_PATH, mode="r", encoding="utf-8") as expected_abi:
        expected_abi_json = json.loads(expected_abi.read())

    resp = rpc_call(
        "starknet_getClass", params={"block_id": "latest", "class_hash": class_hash}
    )
    contract_class = resp["result"]

    assert contract_class["entry_points_by_type"] == EXPECTED_ENTRY_POINTS
    assert isinstance(contract_class["program"], str)
    decompress_program(contract_class["program"])
    assert contract_class["abi"] == expected_abi_json


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_get_class():
    """
    Test get contract class V1
    """
    contract_class, _, compiled_class_hash = load_cairo1_contract()

    declare_response = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(declare_response)
    class_hash = declare_response.json()["class_hash"]

    resp = rpc_call(
        "starknet_getClass", params={"block_id": "latest", "class_hash": class_hash}
    )

    contract_class = resp["result"]
    assert_correct_cairo_1_contract(contract_class)


@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_get_class_at():
    """
    Test get contract class V1
    """
    contract_class, _, compiled_class_hash = load_cairo1_contract()

    declare_response = send_declare_v2(
        contract_class=contract_class,
        compiled_class_hash=compiled_class_hash,
        sender_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        sender_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_declare_v2_accepted(declare_response)
    class_hash = declare_response.json()["class_hash"]

    # deploy
    initial_balance = 10
    deploy_info = deploy(
        class_hash=class_hash,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        inputs=[str(initial_balance)],
        max_fee=int(1e18),
    )
    assert_tx_status(
        tx_hash=deploy_info["tx_hash"], expected_tx_status="ACCEPTED_ON_L2"
    )

    resp = rpc_call(
        "starknet_getClassAt",
        params={"block_id": "latest", "contract_address": deploy_info["address"]},
    )

    contract_class = resp["result"]
    assert_correct_cairo_1_contract(contract_class)


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
def test_get_deprecated_class_at(deploy_info):
    """
    Test get contract class at given contract address
    """
    with open(ABI_PATH, mode="r", encoding="utf-8") as expected_abi:
        expected_abi_json = json.loads(expected_abi.read())

    contract_address: str = deploy_info["address"]
    block_id: BlockId = "latest"

    resp = rpc_call(
        "starknet_getClassAt",
        params={
            "contract_address": rpc_felt(contract_address),
            "block_id": block_id,
        },
    )
    contract_class = resp["result"]

    assert contract_class["entry_points_by_type"] == EXPECTED_ENTRY_POINTS
    assert isinstance(contract_class["program"], str)
    decompress_program(contract_class["program"])
    assert contract_class["abi"] == expected_abi_json
