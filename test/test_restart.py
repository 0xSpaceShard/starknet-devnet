"""
Test restart endpoint
"""

import pytest
import requests

from .account import declare_and_deploy_with_chargeable, invoke
from .settings import APP_URL
from .shared import (
    ABI_PATH,
    CONTRACT_PATH,
    GENESIS_BLOCK_HASH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_transaction_not_received,
    assert_tx_status,
    call,
    devnet_in_background,
    get_block,
)


def restart():
    """Get restart response"""
    return requests.post(f"{APP_URL}/restart")


def get_state_update():
    """Get state update"""
    res = requests.get(f"{APP_URL}/feeder_gateway/get_state_update")
    return res.json()


def deploy_contract(salt=None):
    """Deploy empty contract with balance of 0"""
    return declare_and_deploy_with_chargeable(CONTRACT_PATH, inputs=["0"], salt=salt)


@pytest.mark.restart
@devnet_in_background()
def test_restart_on_initial_state():
    """Performs restart on intact devnet"""
    res = restart()
    assert res.status_code == 200


@pytest.mark.restart
@devnet_in_background("--lite-mode")
def test_restart_on_initial_state_lite():
    """Performs restart on intact devnet in lite mode"""
    res = restart()
    assert res.status_code == 200


@pytest.mark.restart
@devnet_in_background()
def test_transaction():
    """Checks that there is no deploy transaction after the restart"""
    deploy_info = deploy_contract()
    tx_hash = deploy_info["tx_hash"]
    assert_tx_status(tx_hash, "ACCEPTED_ON_L2")

    restart()

    assert_transaction_not_received(tx_hash=tx_hash)


@pytest.mark.restart
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_contract():
    """Checks if contract storage is reset"""
    salt = "0x99"
    deploy_info = deploy_contract(salt)
    contract_address = deploy_info["address"]
    balance = call("get_balance", contract_address, ABI_PATH)
    assert balance == "0"

    invoke(
        calls=[(contract_address, "increase_balance", [10, 20])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
    )
    balance = call("get_balance", contract_address, ABI_PATH)

    assert balance == "30"

    restart()

    deploy_contract(salt)
    balance = call("get_balance", contract_address, ABI_PATH)
    assert balance == "0"


@pytest.mark.restart
@devnet_in_background()
def test_state_update():
    """Checks if state update is reset"""
    deploy_contract()
    state_update = get_state_update()

    assert state_update is not None
    assert state_update["block_hash"] != GENESIS_BLOCK_HASH

    restart()

    state_update = get_state_update()

    assert state_update["block_hash"] == GENESIS_BLOCK_HASH


GAS_PRICE = str(int(1e9))


@devnet_in_background("--gas-price", GAS_PRICE)
def test_gas_price_unaffected_by_restart():
    """Checks that gas price is not affected by restart"""
    deploy_contract()
    block_before = get_block(parse=True)
    gas_price_before = str(int(block_before["gas_price"], 16))
    assert gas_price_before == GAS_PRICE

    restart()

    deploy_contract()
    block_after = get_block(parse=True)
    assert block_after["block_hash"] != block_before["block_hash"]
    gas_price_after = str(int(block_after["gas_price"], 16))
    assert gas_price_after == GAS_PRICE
