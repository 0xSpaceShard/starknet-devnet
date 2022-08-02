"""Test devnet contract deployment"""

from typing import List
import pytest

from starkware.starknet.business_logic.internal_transaction import InternalDeploy
from starkware.starknet.core.os.contract_address.contract_address import calculate_contract_address
from starkware.starknet.definitions import constants
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import Deploy
from starkware.starknet.services.api.feeder_gateway.response_objects import TransactionStatus

from starknet_devnet.starknet_wrapper import StarknetWrapper, DevnetConfig
from .shared import CONTRACT_PATH

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
        version=constants.TRANSACTION_VERSION
    )

@pytest.mark.asyncio
async def test_deploy():
    """
    Test the deployment of a contract.
    """
    devnet = StarknetWrapper(config=DevnetConfig())
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(deploy_transaction=deploy_transaction)

    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition
    )

    assert contract_address == expected_contract_address

    state = await devnet.get_state()

    internal_tx = InternalDeploy.from_external(
        external_tx=deploy_transaction,
        general_config=state.general_config
    )

    assert tx_hash == internal_tx.hash_value

@pytest.mark.asyncio
async def test_deploy_lite():
    """
    Test the deployment of a contract with lite mode.
    """
    devnet = StarknetWrapper(config=DevnetConfig(lite_mode_block_hash=True, lite_mode_deploy_hash=True))
    await devnet.initialize()
    deploy_transaction = get_deploy_transaction(inputs=[0])

    contract_address, tx_hash = await devnet.deploy(deploy_transaction=deploy_transaction)
    expected_contract_address = calculate_contract_address(
        deployer_address=0,
        constructor_calldata=deploy_transaction.constructor_calldata,
        salt=deploy_transaction.contract_address_salt,
        contract_class=deploy_transaction.contract_definition
    )

    assert contract_address == expected_contract_address
    assert tx_hash == 0

    tx_status = devnet.transactions.get_transaction_status(hex(tx_hash))

    assert tx_status["tx_status"] == TransactionStatus.ACCEPTED_ON_L2.name
    assert tx_status["block_hash"] == '0x0'
