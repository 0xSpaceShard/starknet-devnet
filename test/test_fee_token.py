"""Fee token related tests."""

import json
import pytest
import requests
from starknet_devnet.fee_token import FeeToken
from starknet_devnet.server import app

from .account import invoke
from .settings import APP_URL
from .test_account import (
    deploy_empty_contract,
    assert_tx_status,
    get_transaction_receipt,
    get_account_balance,
)
from .shared import (
    EXPECTED_FEE_TOKEN_ADDRESS,
    GENESIS_BLOCK_NUMBER,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import assert_equal, devnet_in_background, get_block, mint


@pytest.mark.fee_token
def test_precomputed_address_unchanged():
    """Assert that the precomputed fee_token address is unchanged."""
    assert_equal(FeeToken.ADDRESS, int(EXPECTED_FEE_TOKEN_ADDRESS, 16))


@pytest.mark.fee_token
def test_fee_token_address():
    """Sends fee token request;"""
    response = app.test_client().get("/fee_token")
    assert response.status_code == 200
    assert response.json.get("address") == EXPECTED_FEE_TOKEN_ADDRESS
    assert response.json.get("symbol") == "ETH"


def mint_client(data: dict):
    """Send mint request to app test client"""
    return app.test_client().post(
        "/mint", content_type="application/json", data=json.dumps(data)
    )


def test_negative_mint():
    """Assert failure if mint amount negative"""
    resp = mint_client({"amount": -10, "address": "0x1"})

    assert resp.status_code == 400
    assert resp.json["message"] == "amount value must be greater than 0."


def test_mint_amount_string():
    """Assert failure if mint amount not int"""
    resp = mint_client({"amount": "abc", "address": "0x1"})

    assert resp.status_code == 400
    assert resp.json["message"] == "amount value must be an integer."


def test_mint_amount_bool():
    """Assert failure if mint amount not int"""
    resp = mint_client({"amount": True, "address": "0x1"})

    assert resp.status_code == 400
    assert resp.json["message"] == "amount value must be an integer."


def test_mint_amount_scientific():
    """Assert failure if mint amount not int"""
    resp = mint_client({"amount": 10e21, "address": "0x1"})

    assert resp.status_code == 200


def test_mint_amount_integer_float():
    """Assert failure if mint amount not int"""
    resp = mint_client({"amount": 12.00, "address": "0x1"})

    assert resp.status_code == 200


def test_missing_mint_amount():
    """Assert failure if mint amount missing"""
    resp = mint_client({"address": "0x1"})

    assert resp.status_code == 400
    assert resp.json["message"] == "amount value must be provided."


def test_wrong_mint_address_format():
    """Assert failure if mint address of wrong format"""
    resp = mint_client({"amount": 10, "address": "invalid_address"})

    assert resp.status_code == 400
    assert resp.json["message"] == "address value must be a hex string."


def test_missing_mint_address():
    """Assert failure if mint address missing"""
    resp = mint_client({"amount": 10})

    assert resp.status_code == 400
    assert resp.json["message"] == "address value must be provided."


@pytest.mark.fee_token
@devnet_in_background()
def test_mint():
    """Assert that mint will increase account balance and latest block created with correct transaction amount"""

    account_address = (
        "0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac4123655502"
    )
    response = mint(address=account_address, amount=50_000)
    assert response.get("new_balance") == 50_000
    assert response.get("unit") == "wei"
    assert response.get("tx_hash").startswith("0x")

    get_block(block_number="latest")
    response = requests.get(f"{APP_URL}/feeder_gateway/get_block?blockNumber=latest")
    assert response.status_code == 200
    assert response.json().get("block_number") == GENESIS_BLOCK_NUMBER + 1
    assert int(response.json().get("transactions")[0].get("calldata")[1], 16) == 50_000


@pytest.mark.fee_token
@devnet_in_background()
def test_mint_lite():
    """Assert that mint lite will increase account balance without producing block"""
    response = mint(
        address="0x34d09711b5c047471fd21d424afbf405c09fd584057e1d69c77223b535cf769",
        amount=50_000,
        lite=True,
    )
    assert response.get("new_balance") == 50000
    assert response.get("unit") == "wei"
    assert response.get("tx_hash") is None


@pytest.mark.fee_token
@devnet_in_background(
    *PREDEPLOY_ACCOUNT_CLI_ARGS,
    "--gas-price",
    "100_000_000",
    "--initial-balance",
    "10",
)
def test_increase_balance():
    """Assert tx failure if insufficient funds; assert tx success after mint"""

    deploy_info = deploy_empty_contract()
    account_address = PREDEPLOYED_ACCOUNT_ADDRESS
    initial_account_balance = get_account_balance(account_address)

    invoke_tx_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", ["10", "20"])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=10**21,
    )

    assert_tx_status(invoke_tx_hash, "REJECTED")
    invoke_receipt = get_transaction_receipt(invoke_tx_hash)
    assert (
        "subtraction overflow"
        in invoke_receipt["transaction_failure_reason"]["error_message"]
    )

    intermediate_account_balance = get_account_balance(account_address)
    assert_equal(initial_account_balance, intermediate_account_balance)

    mint_amount = 200_000_000_000_000
    mint(address=account_address, amount=mint_amount)
    balance_after_mint = get_account_balance(account_address)
    assert_equal(balance_after_mint, initial_account_balance + mint_amount)

    invoke_tx_hash = invoke(
        calls=[(deploy_info["address"], "increase_balance", ["10", "20"])],
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=10**21,
    )  # big enough
    assert_tx_status(invoke_tx_hash, "ACCEPTED_ON_L2")

    invoke_receipt = get_transaction_receipt(invoke_tx_hash)
    actual_fee = int(invoke_receipt["actual_fee"], 16)

    final_account_balance = get_account_balance(account_address)
    assert_equal(
        final_account_balance, initial_account_balance + mint_amount - actual_fee
    )
