"""
Test get_state_update endpoint
"""

import re

import pytest
import requests
from starkware.starknet.core.os.contract_class.deprecated_class_hash import (
    compute_deprecated_class_hash,
)
from starkware.starknet.public.abi import get_selector_from_name

from .account import declare, declare_and_deploy_with_chargeable, invoke
from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    DEPLOYER_CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    EXPECTED_FEE_TOKEN_ADDRESS,
    GENESIS_BLOCK_HASH,
    GENESIS_BLOCK_NUMBER,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    STORAGE_CONTRACT_PATH,
)
from .util import (
    assert_equal,
    assert_hex_equal,
    assert_transaction,
    devnet_in_background,
    get_block,
    load_contract_class,
    mint,
)

STORAGE_KEY = hex(get_selector_from_name("storage"))


def get_state_update_response(block_hash=None, block_number=None):
    """Get state update response"""
    params = {
        "blockHash": block_hash,
        "blockNumber": block_number,
    }

    res = requests.get(f"{APP_URL}/feeder_gateway/get_state_update", params=params)

    return res


def get_state_update(block_hash=None, block_number=None):
    """Get state update"""
    return get_state_update_response(block_hash, block_number).json()


def deploy_empty_contract():
    """
    Deploy storage contract
    Returns contract address.
    """
    deploy_dict = declare_and_deploy_with_chargeable(STORAGE_CONTRACT_PATH)
    contract_address = deploy_dict["address"]

    return contract_address


def get_class_hash_at_path(contract_path: str):
    """Get contract hash of the sample contract"""
    contract_class = load_contract_class(contract_path)
    return compute_deprecated_class_hash(contract_class)


@pytest.mark.state_update
@devnet_in_background()
def test_initial_state_update():
    """Test initial state update"""
    state_update = get_state_update()

    assert_equal(state_update["block_hash"], GENESIS_BLOCK_HASH)


@pytest.mark.state_update
@devnet_in_background()
def test_deployed_contracts():
    """Test deployed contracts in the state update"""
    contract_address = deploy_empty_contract()

    state_update = get_state_update()
    deployed_contracts = state_update["state_diff"]["deployed_contracts"]

    assert_equal(len(deployed_contracts), 1)
    assert_equal(int(deployed_contracts[0]["address"], 16), int(contract_address, 16))

    deployed_contract_hash = deployed_contracts[0]["class_hash"]

    assert_equal(
        int(deployed_contract_hash, 16), get_class_hash_at_path(STORAGE_CONTRACT_PATH)
    )


@pytest.mark.state_update
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_storage_diff():
    """Test storage diffs in the state update"""
    contract_address = deploy_empty_contract()
    contract_address_hex = hex(int(contract_address, 16))

    value = 30
    invoke_tx_hash = invoke(
        calls=[(contract_address, "store_value", [value])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    assert_transaction(invoke_tx_hash, "ACCEPTED_ON_L2")

    state_update = get_state_update()
    storage_diffs = state_update["state_diff"]["storage_diffs"]
    assert storage_diffs.keys() == {EXPECTED_FEE_TOKEN_ADDRESS, contract_address_hex}
    assert storage_diffs[contract_address_hex] == [
        {
            "value": hex(value),
            "key": "0x35fe13a5db37080bfbfae639e6c19be9719e0fbdd4db062eb83cceb4d85a7fe",
        }
    ]


@pytest.mark.state_update
@devnet_in_background()
def test_block_hash():
    """Test block hash in the state update"""
    dummy_address = "0x1"
    dummy_amount = 1

    # generate first block
    mint(address=dummy_address, amount=dummy_amount, lite=False)
    initial_state_update = get_state_update()

    first_block = get_block(parse=True)
    first_block_hash = first_block["block_hash"]

    assert_equal(first_block_hash, initial_state_update["block_hash"])

    # generate second block
    mint(address=dummy_address, amount=dummy_amount, lite=False)

    new_state_update = get_state_update()
    previous_state_update = get_state_update(first_block_hash)

    assert new_state_update["block_hash"] != first_block_hash
    assert_equal(previous_state_update, initial_state_update)


@pytest.mark.state_update
@devnet_in_background()
def test_wrong_block_hash():
    """Test wrong block hash in the state update"""
    state_update_response = get_state_update_response(block_hash="WRONG_HASH")

    assert_equal(state_update_response.status_code, 500)


@pytest.mark.state_update
@devnet_in_background()
def test_block_number():
    """Test block hash in the state update"""
    dummy_address = "0x1"
    dummy_amount = 1

    # generate first block
    mint(address=dummy_address, amount=dummy_amount, lite=False)
    initial_state_update = get_state_update()

    # generate second block
    mint(address=dummy_address, amount=dummy_amount, lite=False)

    new_state_update = get_state_update()
    first_block_state_update = get_state_update(block_number=GENESIS_BLOCK_NUMBER + 1)
    second_block_state_update = get_state_update(block_number=GENESIS_BLOCK_NUMBER + 2)

    assert_equal(first_block_state_update, initial_state_update)
    assert_equal(second_block_state_update, new_state_update)


@pytest.mark.state_update
@devnet_in_background()
def test_wrong_block_number():
    """Test wrong block hash in the state update"""
    state_update_response = get_state_update_response(block_number=42)

    assert_equal(state_update_response.status_code, 500)


@pytest.mark.state_update
@devnet_in_background()
def test_roots():
    """Test new root and old root in the state update"""
    dummy_address = "0x1"
    dummy_amount = 1

    # generate first block
    mint(address=dummy_address, amount=dummy_amount, lite=False)

    state_update = get_state_update()

    new_root = state_update["new_root"]

    assert re.match(r"^[a-fA-F0-9]{64}$", new_root)
    assert state_update["old_root"] is not None

    # generate second block
    mint(address=dummy_address, amount=dummy_amount, lite=False)

    state_update = get_state_update()
    old_root = state_update["old_root"]

    assert_equal(old_root, new_root)


@pytest.mark.state_update
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_declaration_and_deployment():
    """Test if declared classes successfully registered"""
    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=int(4e16),
    )
    contract_class_hash = declare_info["class_hash"]
    assert_hex_equal(contract_class_hash, EXPECTED_CLASS_HASH)

    diff_after_declare = get_state_update()["state_diff"]
    assert diff_after_declare["old_declared_contracts"] == [contract_class_hash]
    assert diff_after_declare["declared_classes"] == []

    # Deploy the deployer - also deploys a contract of the declared class using the deploy syscall
    initial_balance_in_constructor = 5
    deployer_deploy_info = declare_and_deploy_with_chargeable(
        contract=DEPLOYER_CONTRACT_PATH,
        inputs=[int(contract_class_hash, 16), initial_balance_in_constructor],
    )
    deployer_class_hash = hex(get_class_hash_at_path(DEPLOYER_CONTRACT_PATH))
    deployer_address = deployer_deploy_info["address"]

    diff_after_deploy = get_state_update()["state_diff"]
    deployer_diff = diff_after_deploy["deployed_contracts"][0]
    assert_hex_equal(deployer_diff["class_hash"], deployer_class_hash)
    assert_hex_equal(deployer_diff["address"], deployer_address)

    deployed_contract_diff = diff_after_deploy["deployed_contracts"][1]
    assert_hex_equal(deployed_contract_diff["class_hash"], contract_class_hash)
    # deployed_contract_diff["address"] is a random value

    # no declarations in the last block
    assert diff_after_deploy["old_declared_contracts"] == []
    assert diff_after_deploy["declared_classes"] == []
