"""Predeployed account tests"""

import pytest
import requests
from starkware.starknet.core.os.class_hash import compute_class_hash

from starknet_devnet.contract_class_wrapper import (
    DEFAULT_ACCOUNT_HASH_BYTES,
    DEFAULT_ACCOUNT_PATH,
)

from .settings import APP_URL
from .support.assertions import assert_valid_schema
from .util import assert_equal, devnet_in_background, load_contract_class

ACCOUNTS_SEED_DEVNET_ARGS = [
    "--accounts",
    "3",
    "--seed",
    "123",
    "--gas-price",
    "100",
    "--initial-balance",
    "1_000",
]


@pytest.mark.account_predeployed
def test_precomputed_account_hash():
    """Test if the precomputed hash of the account contract is correct."""

    contract_class = load_contract_class(DEFAULT_ACCOUNT_PATH)
    recalculated_hash = compute_class_hash(contract_class=contract_class)
    assert_equal(recalculated_hash, int.from_bytes(DEFAULT_ACCOUNT_HASH_BYTES, "big"))


@pytest.mark.account_predeployed
@devnet_in_background(*ACCOUNTS_SEED_DEVNET_ARGS)
def test_predeployed_accounts_predefined_values():
    """Test if --account --seed --initial-balance return exact calculated values"""
    response = requests.get(f"{APP_URL}/predeployed_accounts")
    assert response.status_code == 200
    assert_valid_schema(response.json(), "predeployed_accounts_fixed_seed.json")
