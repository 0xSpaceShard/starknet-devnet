"""
This module introduces `LiteStarknetState`, optimized lite-mode version of StarknetState.
"""
from typing import List, Tuple, Union, Optional

from starkware.starknet.definitions import constants, fields
from starkware.starknet.testing.starknet import (
    Starknet,
    StarknetState,
)
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.business_logic.execution.objects import TransactionExecutionInfo

from lite_mode.lite_internal_deploy import LiteInternalDeploy

CastableToAddressSalt = Union[str, int]

# pylint: disable=arguments-differ, too-many-arguments
class LiteStarknetState(StarknetState):
    """
    The lite version of StarknetState which avoid transaction hash calculation in deploy.
    """

    async def deploy(
        self,
        contract_class: ContractClass,
        constructor_calldata: List[int],
        starknet: Starknet,
        tx_number: int,
        contract_address_salt: Optional[CastableToAddressSalt] = None,
    ) -> Tuple[int, TransactionExecutionInfo]:
        if contract_address_salt is None:
            contract_address_salt = fields.ContractAddressSalt.get_random_value()
        if isinstance(contract_address_salt, str):
            contract_address_salt = int(contract_address_salt, 16)
        assert isinstance(contract_address_salt, int)

        transaction = LiteInternalDeploy.lite_create(
            contract_address_salt=contract_address_salt,
            constructor_calldata=constructor_calldata,
            contract_class=contract_class,
            version=constants.TRANSACTION_VERSION,
            tx_number=tx_number,
        )

        await starknet.state.state.set_contract_class(
            class_hash=transaction.contract_hash, contract_class=contract_class
        )
        tx_execution_info = await starknet.state.execute_tx(tx=transaction)

        return transaction.contract_address, tx_execution_info
