"""
Account class and its predefined constants.
"""

from starkware.cairo.lang.vm.crypto import pedersen_hash
from starkware.solidity.utils import load_nearby_contract
from starkware.starknet.business_logic.state.objects import ContractState, ContractCarriedState
from starkware.starknet.public.abi import get_selector_from_name
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.core.os.contract_address.contract_address import calculate_contract_address_from_hash
from starkware.starknet.storage.starknet_storage import StorageLeaf
from starkware.starknet.testing.contract import StarknetContract
from starkware.python.utils import to_bytes

from starknet_devnet.util import Uint256

class Account:
    """Account contract wrapper."""

    CONTRACT_CLASS: ContractClass = None # loaded lazily
    CONTRACT_PATH = "accounts_artifacts/OpenZeppelin/0.3.1/Account.cairo/Account"

    # Precalculated to save time
    # HASH = compute_class_hash(contract_class=Account.get_contract_class()))
    HASH = 580711710156617243550448398501018980467831526895029280465303474122300077395
    HASH_BYTES = to_bytes(HASH)

    def __init__(self, starknet_wrapper, private_key: int, public_key: int, initial_balance: int):
        self.starknet_wrapper = starknet_wrapper
        self.private_key = private_key
        self.public_key = public_key

        # salt and class_hash have frozen values that make the constructor_calldata
        # the only thing that affects the account address
        self.address = calculate_contract_address_from_hash(
            salt=20,
            class_hash=1803505466663265559571280894381905521939782500874858933595227108099796801620,
            constructor_calldata=[public_key],
            deployer_address=0
        )
        self.initial_balance = initial_balance

    @classmethod
    def get_contract_class(cls):
        """Returns contract class via lazy loading."""
        if not cls.CONTRACT_CLASS:
            cls.CONTRACT_CLASS = ContractClass.load(load_nearby_contract(cls.CONTRACT_PATH))
        return cls.CONTRACT_CLASS

    def to_json(self):
        """Return json account"""
        return {
            "initial_balance": self.initial_balance,
            "private_key": hex(self.private_key),
            "public_key": hex(self.public_key),
            "address": hex(self.address)
        }

    async def deploy(self) -> StarknetContract:
        """Deploy this account."""
        starknet = self.starknet_wrapper.starknet
        contract_class = Account.get_contract_class()
        account_carried_state = starknet.state.state.contract_states[self.address]
        account_state = account_carried_state.state
        assert not account_state.initialized

        starknet.state.state.contract_definitions[Account.HASH_BYTES] = contract_class

        newly_deployed_account_state = await ContractState.create(
            contract_hash=Account.HASH_BYTES,
            storage_commitment_tree=account_state.storage_commitment_tree
        )

        starknet.state.state.contract_states[self.address] = ContractCarriedState(
            state=newly_deployed_account_state,
            storage_updates={
                get_selector_from_name("Account_public_key"): StorageLeaf(self.public_key)
            }
        )

        # set initial balance
        fee_token_address = starknet.state.general_config.fee_token_address
        fee_token_storage_updates = starknet.state.state.contract_states[fee_token_address].storage_updates

        balance_address = pedersen_hash(get_selector_from_name("ERC20_balances"), self.address)
        initial_balance_uint256 = Uint256.from_felt(self.initial_balance)
        fee_token_storage_updates[balance_address] = StorageLeaf(initial_balance_uint256.low)
        fee_token_storage_updates[balance_address + 1] = StorageLeaf(initial_balance_uint256.high)

        contract =  StarknetContract(
            state=starknet.state,
            abi=contract_class.abi,
            contract_address=self.address,
            deploy_execution_info=None
        )

        self.starknet_wrapper.store_contract(self.address, contract, contract_class)
