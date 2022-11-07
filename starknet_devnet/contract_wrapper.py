"""
Contains code for wrapping StarknetContract instances.
"""

from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.testing.contract import StarknetContract


class ContractWrapper:
    """
    Wraps a StarknetContract and its ContractClass.
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
