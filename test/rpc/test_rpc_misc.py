"""
Tests RPC miscellaneous
"""

from __future__ import annotations

from test.account import declare, invoke
from test.rpc.rpc_utils import deploy_and_invoke_storage_contract, rpc_call
from test.rpc.test_data.get_events import GET_EVENTS_TEST_DATA
from test.shared import (
    CONTRACT_PATH,
    DEPLOYER_CONTRACT_PATH,
    EVENTS_CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    EXPECTED_FEE_TOKEN_ADDRESS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from test.test_account import deploy_empty_contract
from test.test_state_update import get_class_hash_at_path
from test.util import assert_hex_equal, assert_transaction, deploy

import pytest
from starkware.starknet.public.abi import get_storage_var_address

from starknet_devnet.blueprints.rpc.utils import rpc_felt
from starknet_devnet.general_config import DEFAULT_GENERAL_CONFIG


@pytest.fixture(name="input_data")
def fixture_input_data(request):
    """
    Fixture for input data
    """
    return request.param


@pytest.fixture(name="expected_data")
def fixture_expected_data(request):
    """
    Fixture for return expected data
    """
    return request.param


@pytest.mark.usefixtures("devnet_with_account")
def test_get_state_update():
    """Test if declared classes successfully registered"""

    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    contract_class_hash = declare_info["class_hash"]
    assert_hex_equal(contract_class_hash, EXPECTED_CLASS_HASH)

    resp = rpc_call("starknet_getStateUpdate", params={"block_id": "latest"})
    diff_after_declare = resp["result"]["state_diff"]
    assert diff_after_declare["declared_contract_hashes"] == [
        rpc_felt(contract_class_hash)
    ]

    # Deploy the deployer - also deploys a contract of the declared class using the deploy syscall
    initial_balance_in_constructor = "5"
    deployer_deploy_info = deploy(
        contract=DEPLOYER_CONTRACT_PATH,
        inputs=[contract_class_hash, initial_balance_in_constructor],
    )
    deployer_class_hash = hex(get_class_hash_at_path(DEPLOYER_CONTRACT_PATH))
    deployer_address = deployer_deploy_info["address"]

    resp = rpc_call("starknet_getStateUpdate", params={"block_id": "latest"})
    diff_after_deploy = resp["result"]["state_diff"]

    deployer_diff = diff_after_deploy["deployed_contracts"][0]
    assert_hex_equal(deployer_diff["class_hash"], deployer_class_hash)
    assert_hex_equal(deployer_diff["address"], deployer_address)

    deployed_contract_diff = diff_after_deploy["deployed_contracts"][1]
    assert_hex_equal(deployed_contract_diff["class_hash"], contract_class_hash)
    # deployed_contract_diff["address"] is a random value

    # deployer expected to be declared
    assert diff_after_deploy["declared_contract_hashes"] == [
        rpc_felt(deployer_class_hash)
    ]


@pytest.mark.usefixtures("devnet_with_account")
def test_storage_diff():
    """Test storage diffs in the state update"""

    value = 30
    contract_address, _ = deploy_and_invoke_storage_contract(value)

    resp = rpc_call("starknet_getStateUpdate", params={"block_id": "latest"})
    storage_diffs = resp["result"]["state_diff"]["storage_diffs"]

    # list can be in different order per test run
    if storage_diffs[0]["address"] == rpc_felt(contract_address):
        storage_diffs[0], storage_diffs[1] = storage_diffs[1], storage_diffs[0]

    assert storage_diffs[0]["address"] == rpc_felt(EXPECTED_FEE_TOKEN_ADDRESS)
    assert storage_diffs[1] == {
        "address": rpc_felt(contract_address),
        "storage_entries": [
            {
                "key": rpc_felt(get_storage_var_address("storage")),
                "value": rpc_felt(value),
            }
        ],
    }


@pytest.mark.parametrize("params", [{}, None])
@pytest.mark.usefixtures("run_devnet_in_background")
def test_chain_id(params):
    """
    Test chain id
    """
    chain_id = DEFAULT_GENERAL_CONFIG.chain_id.value

    resp = rpc_call("starknet_chainId", params=params)
    rpc_chain_id = resp["result"]

    assert rpc_chain_id == hex(chain_id)


@pytest.mark.parametrize("params", [{}, None])
@pytest.mark.usefixtures("run_devnet_in_background")
def test_syncing(params):
    """
    Test syncing
    """
    resp = rpc_call("starknet_syncing", params=params)
    assert "result" in resp, f"Unexpected response: {resp}"
    assert resp["result"] is False


@pytest.mark.parametrize("params", [2, "random string", True])
@pytest.mark.usefixtures("run_devnet_in_background")
def test_call_with_invalid_params(params):
    """Call with invalid params"""

    # could be any legal method, just passing something to get params to fail
    ex = rpc_call(method="starknet_getClass", params=params)
    assert ex["error"] == {"code": -32602, "message": "Invalid params"}


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, input_data, expected_data",
    GET_EVENTS_TEST_DATA,
    indirect=True,
)
def test_get_events(input_data, expected_data):
    """
    Test RPC get_events.
    """
    deploy_info = deploy(EVENTS_CONTRACT_PATH)
    for i in range(2):
        invoke(
            calls=[(deploy_info["address"], "increase_balance", [i])],
            account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
            private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        )
    resp = rpc_call("starknet_getEvents", params=input_data)
    assert len(expected_data) == len(resp["result"]["events"])
    for i, data in enumerate(expected_data):
        assert str(resp["result"]["events"][i]["data"]) == str(data)

    if "continuation_token" in input_data:
        expected_continuation_token = int(input_data["continuation_token"])

        # increase continuation_token when events are not empty
        if resp["result"]["events"]:
            expected_continuation_token += 1

        assert expected_continuation_token == int(resp["result"]["continuation_token"])


@pytest.mark.usefixtures("devnet_with_account")
def test_get_nonce():
    """Test get_nonce"""

    account_address = PREDEPLOYED_ACCOUNT_ADDRESS

    initial_resp = rpc_call(
        method="starknet_getNonce",
        params={"block_id": "latest", "contract_address": rpc_felt(account_address)},
    )
    assert initial_resp["result"] == "0x00"

    deployment_info = deploy_empty_contract()

    invoke_tx_hash = invoke(
        calls=[(deployment_info["address"], "increase_balance", [10, 20])],
        account_address=account_address,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    final_resp = rpc_call(
        method="starknet_getNonce",
        params={"block_id": "latest", "contract_address": rpc_felt(account_address)},
    )
    assert final_resp["result"] == "0x01"


@pytest.mark.usefixtures("devnet_with_account")
def test_get_nonce_invalid_address():
    """Test get_nonce with invalid address"""

    account_address = "0x1111"

    ex = rpc_call(
        method="starknet_getNonce",
        params={"block_id": "latest", "contract_address": rpc_felt(account_address)},
    )
    assert ex["error"] == {"code": 20, "message": "Contract not found"}
