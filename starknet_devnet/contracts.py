"""
Class for storing and handling contracts
"""

from typing import Dict

from .contract_wrapper import ContractWrapper


class DevnetContracts:
    """
    This class is used to store the deployed contracts of the devnet.
    """

    def __init__(self):
        self.__instances: Dict[int, ContractWrapper] = {}

    def store(self, address: int, contract_wrapper: ContractWrapper) -> None:
        """
        Store the contract wrapper.
        """
        self.__instances[address] = contract_wrapper

    # TODO replace with state.get_class_hash_at
    def is_deployed(self, address: int) -> bool:
        """
        Check if the contract is deployed.
        """
        assert isinstance(address, int)
        return address in self.__instances

    def get_deployment_tx_hash(self, address: int) -> int:
        """
        Return deployment tx hash given the contract address
        """

        assert isinstance(address, int)
        return self.__instances[address].deployment_tx_hash
