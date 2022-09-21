"""Test devnet contract deployment"""

from typing import List
import pytest

from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address,
)
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Deploy
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
)

from starknet_devnet.devnet_config import parse_args, DevnetConfig
from starknet_devnet.starknet_wrapper import StarknetWrapper
from .util import assert_hex_equal
from .shared import (
    CONTRACT_PATH,
    SUPPORTED_TX_VERSION,
    PREDEPLOY_ACCOUNT_CLI_ARGS,
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


@pytest.fixture(name="starknet_wrapper_args")
def fixture_starknet_wrapper_args(request):
    """
    Fixture to return values of dev net arguments
    """
    return request.param


@pytest.mark.parametrize(
    "starknet_wrapper_args, expected_tx_hash, expected_block_hash",
    [
        (
            [*PREDEPLOY_ACCOUNT_CLI_ARGS],
            "0x615badf1d4446082f598fa16416d4d3623dfb8cc5d58276515f502f8fa22009",
            "",
        ),
        (
            [*PREDEPLOY_ACCOUNT_CLI_ARGS, "--lite-mode"],
            "0x0",
            "0x1",
        ),
    ],
    indirect=True,
)
@pytest.mark.asyncio
async def test_deploy(starknet_wrapper_args, expected_tx_hash, expected_block_hash):
    """
    Test the deployment of a contract.
    """
    devnet = StarknetWrapper(config=DevnetConfig(parse_args(starknet_wrapper_args)))
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(
        deploy_transaction=deploy_transaction,
    )
    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition,
    )

    assert_hex_equal(
        hex(tx_hash),
        expected_tx_hash,
    )
    assert contract_address == expected_contract_address

    tx_status = devnet.transactions.get_transaction_status(hex(tx_hash))
    assert tx_status["tx_status"] == TransactionStatus.ACCEPTED_ON_L2.name

    if "--lite-mode" in starknet_wrapper_args:
        assert tx_status["block_hash"] == expected_block_hash
