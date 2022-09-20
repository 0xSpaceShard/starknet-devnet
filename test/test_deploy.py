"""Test devnet contract deployment"""

from typing import List
import pytest

from starkware.starknet.business_logic.transaction.objects import InternalDeploy
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
from .shared import CONTRACT_PATH, GENESIS_BLOCK_NUMBER, SUPPORTED_TX_VERSION
from .util import assert_hex_equal


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


@pytest.mark.asyncio
async def test_deploy():
    """
    Test the deployment of a contract.
    """
    devnet = StarknetWrapper(config=DevnetConfig(parse_args([])))
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(
        deploy_transaction=deploy_transaction
    )

    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition,
    )

    assert contract_address == expected_contract_address

    state = devnet.get_state()

    internal_tx = InternalDeploy.from_external(
        external_tx=deploy_transaction, general_config=state.general_config
    )

    assert tx_hash == internal_tx.hash_value


@pytest.mark.asyncio
async def test_deploy_lite():
    """
    Test the deployment of a contract with lite mode.
    """
    devnet = StarknetWrapper(config=DevnetConfig(parse_args(["--lite-mode"])))
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(
        deploy_transaction=deploy_transaction
    )
    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition,
    )

    # Currently in lite mode hashes are actually calculated
    assert_hex_equal(
        hex(tx_hash),
        "0x615badf1d4446082f598fa16416d4d3623dfb8cc5d58276515f502f8fa22009",
    )
    assert contract_address == expected_contract_address

    tx_status = devnet.transactions.get_transaction_status(hex(tx_hash))

    assert tx_status["tx_status"] == TransactionStatus.ACCEPTED_ON_L2.name
    assert tx_status["block_hash"] == hex(GENESIS_BLOCK_NUMBER + 1)
