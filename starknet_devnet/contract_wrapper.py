"""
Contains code for wrapping StarknetContract instances.
"""

from typing import List

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.utils.api_utils import cast_to_felts


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
        signature: List[int],
        caller_address: int,
        max_fee: int,
    ):
        """
        Calls the function identified with `entry_point_selector`, potentially passing in `calldata` and `signature`.
        """

        call_info, _ = await self.contract.state.call_raw(
            calldata=calldata,
            caller_address=caller_address,
            contract_address=self.contract.contract_address,
            max_fee=max_fee,
            selector=entry_point_selector,
            signature=signature and cast_to_felts(values=signature),
        )

        result = list(map(hex, call_info.retdata))

        return result

    async def invoke(
        self,
        entry_point_selector: int,
        calldata: List[int],
        signature: List[int],
        caller_address: int,
        max_fee: int,
    ):
        """
        Invokes the function identified with `entry_point_selector`, potentially passing in `calldata` and `signature`.
        """

        execution_info = await self.contract.state.invoke_raw(
            contract_address=self.contract.contract_address,
            selector=entry_point_selector,
            calldata=calldata,
            caller_address=caller_address,
            max_fee=max_fee,
            signature=signature and cast_to_felts(values=signature),
        )

        result = list(map(hex, execution_info.call_info.retdata))
        return result, execution_info
