"""
Contains code for wrapping StarknetContract instances.
"""

from typing import List

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.testing.contract import StarknetContract


class ContractWrapper:
    """
    Wraps a StarknetContract, storing its types and code for later use.
    """

    def __init__(
        self,
        contract: StarknetContract,
        contract_class: ContractClass,
        deployment_tx_hash: int = None,
    ):
        self.contract: StarknetContract = contract
        self.contract_class = contract_class.remove_debug_info()
        self.deployment_tx_hash = deployment_tx_hash

        self.code: dict = {
            "abi": contract_class.abi,
            "bytecode": self.contract_class.dump()["program"]["data"],
        }

    # pylint: disable=too-many-arguments
    async def call(
        self,
        entry_point_selector: int,
        calldata: List[int],
        caller_address: int,
    ):
        """
        Calls the function identified with `entry_point_selector`, potentially passing in `calldata` and `signature`.
        """

        call_info = await self.contract.state.copy().execute_entry_point_raw(
            contract_address=self.contract.contract_address,
            selector=entry_point_selector,
            calldata=calldata,
            caller_address=caller_address,
        )

        result = list(map(hex, call_info.retdata))

        return result
