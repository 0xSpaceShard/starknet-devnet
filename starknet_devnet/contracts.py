"""
Class for storing and handling contracts
"""

from typing import Dict

from .origin import Origin
from .contract_wrapper import ContractWrapper


class DevnetContracts:
    """
    This class is used to store the deployed contracts of the devnet.
    """

    def __init__(self, origin: Origin):
        self.origin = origin
        self.__instances: Dict[int, ContractWrapper] = {}

    def store(
        self, address: int, contract_wrapper: ContractWrapper
    ) -> None:
        """
        Store the contract wrapper.
        """
        self.__instances[address] = contract_wrapper

    def is_deployed(self, address: int) -> bool:
        """
        Check if the contract is deployed.
        """
        assert isinstance(address, int)
        return address in self.__instances
