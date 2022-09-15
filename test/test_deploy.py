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
from .util import (
    assert_equal,
    get_block,
    assert_hex_equal
)
from .shared import (
    CONTRACT_PATH,
    EXPECTED_SALTY_DEPLOY_HASH,
    EXPECTED_SALTY_DEPLOY_HASH_LITE_MODE,
    EXPECTED_SALTY_DEPLOY_BLOCK_HASH_LITE_MODE,
    GENESIS_BLOCK_NUMBER,
    SUPPORTED_TX_VERSION,
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

@pytest.mark.parametrize(
    "dev_net_args, expected_tx_hash, expected_block_hash",
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
@pytest.mark.asyncio
async def test_deploy(dev_net_args, expected_tx_hash, expected_block_hash):
    """
    Test the deployment of a contract.
    """
    devnet = StarknetWrapper(config=DevnetConfig(parse_args(dev_net_args)))
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

    print("contract_address", hex(contract_address))
    print("tx_hash", hex(tx_hash))
    print("expected_contract_address", hex(expected_contract_address))

    # TODO: this can be changed?
    # check if in lite mode expected block hash is 0x1
    # if expected_block_hash == EXPECTED_SALTY_DEPLOY_BLOCK_HASH_LITE_MODE:
    #     assert_equal(expected_block_hash, get_block(parse=True)["block_hash"])
    
    # state = devnet.get_state()
    # if dev_net_args == "--lite-mode":
    #     pass
    # else
    #     internal_tx = InternalDeploy.from_external(
    #         external_tx=deploy_transaction, general_config=state.general_config
    #     )

    # # is this needed?
    assert_hex_equal(
        hex(tx_hash),
        expected_tx_hash,
    )

    assert contract_address == expected_contract_address

    tx_status = devnet.transactions.get_transaction_status(hex(tx_hash))
    assert tx_status["tx_status"] == TransactionStatus.ACCEPTED_ON_L2.name
    # assert tx_status["block_hash"] == hex(GENESIS_BLOCK_NUMBER + 1) # TODO: fix for lite mode
