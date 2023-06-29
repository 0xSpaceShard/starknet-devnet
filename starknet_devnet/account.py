"""
Account class and its predefined constants.
"""

from typing import Optional

from starkware.starknet.business_logic.state.storage_domain import StorageDomain
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.testing.starknet import Starknet

from starknet_devnet.account_util import set_balance
from starknet_devnet.contract_class_wrapper import CompiledClassWrapper
from starknet_devnet.predeployed_contract_wrapper import PredeployedContractWrapper


# pylint: disable=too-many-instance-attributes
class Account(PredeployedContractWrapper):
    """Account contract wrapper."""

    # pylint: disable=too-many-arguments
    def __init__(
        self,
        starknet_wrapper,
        private_key: int,
        public_key: int,
        initial_balance: int,
        account_class_wrapper: CompiledClassWrapper,
        index: Optional[int] = None,
    ):
        self.starknet_wrapper = starknet_wrapper
        self.private_key = private_key
        self.public_key = public_key
        self.contract_class = account_class_wrapper.contract_class
        self.class_hash = account_class_wrapper.hash

        # salt and class_hash have frozen values that make the constructor_calldata
        # the only thing that affects the account address
        self.address = calculate_contract_address_from_hash(
            salt=20,
            class_hash=0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854,
            constructor_calldata=[public_key],
            deployer_address=0,
        )
        self.initial_balance = initial_balance

        self.__index = index
        """Index used when logging/displaying account to user on startup"""

    def to_json(self):
        """Return json account"""
        return {
            "initial_balance": self.initial_balance,
            "private_key": hex(self.private_key),
            "public_key": hex(self.public_key),
            "address": hex(self.address),
        }

    async def _mimic_constructor(self):
        starknet: Starknet = self.starknet_wrapper.starknet

        await starknet.state.state.set_storage_at(
            storage_domain=StorageDomain.ON_CHAIN,
            contract_address=self.address,
            key=get_selector_from_name("Account_public_key"),
            value=self.public_key,
        )

        await set_balance(starknet.state, self.address, self.initial_balance)

    def print(self):
        print(f"Account #{self.__index}:")
        print(f"Address: {hex(self.address)}")
        print(f"Public key: {hex(self.public_key)}")
        print(f"Private key: {hex(self.private_key)}")
        print(flush=True)
