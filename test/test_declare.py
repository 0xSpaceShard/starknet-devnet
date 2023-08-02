"""
Tests of contract class declaration and deploy syscall.
"""

import pytest
import requests

from .account import declare
from .settings import APP_URL
from .shared import (
    CONTRACT_PATH,
    EXPECTED_CLASS_HASH,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
    PREDEPLOYED_ACCOUNT_ADDRESS,
    PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
)
from .util import (
    assert_class_by_hash,
    assert_hex_equal,
    assert_tx_status,
    assert_undeclared_class,
    devnet_in_background,
    get_compiled_class_by_class_hash,
)


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_declare_max_fee_too_low():
    """Test declaring if max fee too low"""

    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=1,
    )
    class_hash = declare_info["class_hash"]
    assert_hex_equal(class_hash, EXPECTED_CLASS_HASH)
    assert_tx_status(declare_info["tx_hash"], "REVERTED")

    assert_undeclared_class(
        resp=requests.get(
            f"{APP_URL}/feeder_gateway/get_class_by_hash?classHash={class_hash}"
        )
    )


@pytest.mark.declare
@devnet_in_background(*PREDEPLOY_ACCOUNT_CLI_ARGS)
def test_declare_happy_path():
    """Test declaring if max fee sufficient"""

    declare_info = declare(
        contract_path=CONTRACT_PATH,
        account_address=PREDEPLOYED_ACCOUNT_ADDRESS,
        private_key=PREDEPLOYED_ACCOUNT_PRIVATE_KEY,
        max_fee=int(1e18),
    )
    class_hash = declare_info["class_hash"]
    assert_hex_equal(class_hash, EXPECTED_CLASS_HASH)
    assert_tx_status(declare_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_class_by_hash(class_hash, CONTRACT_PATH)

    assert_undeclared_class(resp=get_compiled_class_by_class_hash(class_hash))
