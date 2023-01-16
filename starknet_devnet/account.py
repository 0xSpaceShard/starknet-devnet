"""
Account class and its predefined constants.
"""

from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.testing.starknet import Starknet

from starknet_devnet.account_util import set_balance
from starknet_devnet.contract_class_wrapper import ContractClassWrapper


class Account:
    """Account contract wrapper."""

    # pylint: disable=too-many-arguments
    def __init__(
        self,
        starknet_wrapper,
        private_key: int,
        public_key: int,
        initial_balance: int,
        account_class_wrapper: ContractClassWrapper,
    ):
        self.starknet_wrapper = starknet_wrapper
        self.private_key = private_key
        self.public_key = public_key
        self.contract_class = account_class_wrapper.contract_class
        self.class_hash_bytes = account_class_wrapper.hash_bytes

        # salt and class_hash have frozen values that make the constructor_calldata
        # the only thing that affects the account address
        self.address = calculate_contract_address_from_hash(
            salt=20,
            class_hash=0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854,
            constructor_calldata=[public_key],
            deployer_address=0,
        )
        self.initial_balance = initial_balance

    def to_json(self):
        """Return json account"""
        return {
            "initial_balance": self.initial_balance,
            "private_key": hex(self.private_key),
            "public_key": hex(self.public_key),
            "address": hex(self.address),
        }

    async def deploy(self) -> StarknetContract:
        """Deploy this account and set its balance."""
        starknet: Starknet = self.starknet_wrapper.starknet
        contract_class = self.contract_class
        await starknet.state.state.set_contract_class(
            self.class_hash_bytes, contract_class
        )
        await starknet.state.state.deploy_contract(self.address, self.class_hash_bytes)

        await starknet.state.state.set_storage_at(
            self.address, get_selector_from_name("Account_public_key"), self.public_key
        )

        await set_balance(starknet.state, self.address, self.initial_balance)
