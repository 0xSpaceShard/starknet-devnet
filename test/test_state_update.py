"""
Test get_state_update endpoint
"""

import pytest
import requests

from starkware.starknet.core.os.class_hash import compute_class_hash
from starkware.starknet.public.abi import get_selector_from_name

from .util import (
    deploy,
    invoke,
    load_contract_class,
    devnet_in_background,
    get_block,
    assert_equal,
)
from .settings import APP_URL
from .shared import (
    GENESIS_BLOCK_HASH,
    STORAGE_CONTRACT_PATH,
    STORAGE_ABI_PATH,
    GENESIS_BLOCK_NUMBER,
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
    deploy_dict = deploy(STORAGE_CONTRACT_PATH)
    contract_address = deploy_dict["address"]

    return contract_address


def get_contract_hash():
    """Get contract hash of the sample contract"""
    contract_class = load_contract_class(STORAGE_CONTRACT_PATH)
    return compute_class_hash(contract_class)


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

    assert_equal(int(deployed_contract_hash, 16), get_contract_hash())


@pytest.mark.state_update
@devnet_in_background()
def test_storage_diff():
    """Test storage diffs in the state update"""
    contract_address = deploy_empty_contract()
    address = hex(int(contract_address, 16))
    invoke("store_value", ["30"], contract_address, STORAGE_ABI_PATH)

    state_update = get_state_update()
    storage_diffs = state_update["state_diff"]["storage_diffs"]
    assert_equal(len(storage_diffs), 1)

    contract_storage_diffs = storage_diffs[address]

    assert_equal(len(contract_storage_diffs), 1)
    assert_equal(contract_storage_diffs[0]["value"], hex(30))
    assert_equal(contract_storage_diffs[0]["key"], STORAGE_KEY)

    invoke("store_value", ["0"], contract_address, STORAGE_ABI_PATH)

    state_update = get_state_update()
    storage_diffs = state_update["state_diff"]["storage_diffs"]
    contract_storage_diffs = storage_diffs[address]

    assert_equal(len(contract_storage_diffs), 1)
    assert_equal(contract_storage_diffs[0]["value"], hex(0))
    assert_equal(contract_storage_diffs[0]["key"], STORAGE_KEY)


@pytest.mark.state_update
@devnet_in_background()
def test_block_hash():
    """Test block hash in the state update"""
    deploy_empty_contract()
    initial_state_update = get_state_update()

    first_block = get_block(parse=True)
    first_block_hash = first_block["block_hash"]

    assert_equal(first_block_hash, initial_state_update["block_hash"])

    # creates new block
    deploy_empty_contract()

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
    deploy_empty_contract()
    initial_state_update = get_state_update()

    # creates new block
    deploy_empty_contract()

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
    deploy_empty_contract()
    state_update = get_state_update()

    new_root = state_update["new_root"]

    assert new_root is not None
    assert state_update["old_root"] is not None

    # creates new block
    deploy_empty_contract()

    state_update = get_state_update()
    old_root = state_update["old_root"]

    assert_equal(old_root, new_root)
