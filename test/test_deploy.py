"""Test devnet contract deployment"""

from typing import List
import pytest

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Deploy
from starknet_devnet.constants import SUPPORTED_TX_VERSION

from .util import (
    assert_contract_class,
    assert_negative_block_input,
    assert_transaction_not_received,
    assert_transaction_receipt_not_received,
    assert_block,
    assert_contract_code,
    assert_equal,
    assert_receipt,
    assert_salty_deploy,
    assert_storage,
    assert_transaction,
    assert_tx_status,
    deploy,
    get_class_by_hash,
    get_class_hash_at,
    get_full_contract,
    get_block,
)

from .shared import (
    BALANCE_KEY,
    CONTRACT_PATH,
    EVENTS_CONTRACT_PATH,
    EXPECTED_SALTY_DEPLOY_ADDRESS,
    EXPECTED_SALTY_DEPLOY_HASH,
    EXPECTED_SALTY_DEPLOY_HASH_LITE_MODE,
    EXPECTED_SALTY_DEPLOY_BLOCK_HASH_LITE_MODE,
    GENESIS_BLOCK_NUMBER,
    NONEXISTENT_TX_HASH,
)


def get_contract_class():
    """Get the contract class from the contract.json file."""
    with open(CONTRACT_PATH, "r", encoding="utf-8") as contract_class_file:
        return ContractClass.loads(contract_class_file.read())


def get_deploy_transaction(inputs: List[int], salt=0):
    """Get a Deploy transaction."""
    contract_class = get_contract_class()

    return Deploy(
        contract_address_salt=salt,
        contract_definition=contract_class,
        constructor_calldata=inputs,
        version=SUPPORTED_TX_VERSION,
    )


@pytest.mark.usefixtures("run_devnet_in_background")
@pytest.mark.parametrize(
    "run_devnet_in_background, expected_tx_hash, expected_block_hash",
    [
        ([], EXPECTED_SALTY_DEPLOY_HASH, ""),
        (
            ["--lite-mode"],
            EXPECTED_SALTY_DEPLOY_HASH_LITE_MODE,
            EXPECTED_SALTY_DEPLOY_BLOCK_HASH_LITE_MODE,
        ),
    ],
    indirect=True,
)
def test_deploy(expected_tx_hash, expected_block_hash):
    """
    Test the deployment of a contract.
    """
    deploy_info = deploy(CONTRACT_PATH, ["0"])

    assert_tx_status(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction(deploy_info["tx_hash"], "ACCEPTED_ON_L2")
    assert_transaction_not_received(NONEXISTENT_TX_HASH)

    # check storage after deployment
    assert_storage(deploy_info["address"], BALANCE_KEY, "0x0")

    # check block and receipt after deployment
    assert_negative_block_input()

    # check if in lite mode expected block hash is 0x1
    if expected_block_hash == EXPECTED_SALTY_DEPLOY_BLOCK_HASH_LITE_MODE:
        assert_equal(expected_block_hash, get_block(parse=True)["block_hash"])

    assert_block(GENESIS_BLOCK_NUMBER + 1, deploy_info["tx_hash"])
    assert_receipt(deploy_info["tx_hash"], "test/expected/deploy_receipt.json")
    assert_transaction_receipt_not_received(NONEXISTENT_TX_HASH)

    # check code
    assert_contract_code(deploy_info["address"])

    # check contract class
    class_by_address = get_full_contract(deploy_info["address"])
    assert_contract_class(class_by_address, CONTRACT_PATH)

    # check contract class through class hash
    class_hash = get_class_hash_at(deploy_info["address"])
    class_by_hash = get_class_by_hash(class_hash)
    assert_equal(class_by_address, class_by_hash)

    assert_salty_deploy(
        contract_path=EVENTS_CONTRACT_PATH,
        salt="0x99",
        inputs=None,
        expected_status="ACCEPTED_ON_L2",
        expected_address=EXPECTED_SALTY_DEPLOY_ADDRESS,
        expected_tx_hash=expected_tx_hash,
    )
