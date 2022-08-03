"""
Fee token and its predefined constants.
"""

from starkware.solidity.utils import load_nearby_contract
from starkware.starknet.services.api.contract_class import ContractClass
from starkware.starknet.services.api.gateway.transaction import InvokeFunction
from starkware.starknet.storage.starknet_storage import StorageLeaf
from starkware.starknet.business_logic.state.objects import (ContractState, ContractCarriedState)
from starkware.starknet.testing.contract import StarknetContract
from starkware.python.utils import to_bytes
from starkware.starknet.compiler.compile import get_selector_from_name
from starknet_devnet.util import Uint256

class FeeToken:
    """Wrapper of token for charging fees."""

    CONTRACT_CLASS: ContractClass = None # loaded lazily

    # Precalculated
    # HASH = to_bytes(compute_class_hash(contract_class=FeeToken.get_contract_class()))
    HASH = 3000409729603134799471314790024123407246450023546294072844903167350593031855
    HASH_BYTES = to_bytes(HASH)

    # Precalculated to fixed address
    # ADDRESS = calculate_contract_address_from_hash(salt=10, class_hash=HASH,
    # constructor_calldata=[], caller_address=0)
    ADDRESS = 2774287484619332564597403632816768868845110259953541691709975889937073775752
    SYMBOL = "ETH"
    NAME = "ether"

    contract: StarknetContract = None

    def __init__(self, starknet_wrapper):
        self.starknet_wrapper = starknet_wrapper

    @classmethod
    def get_contract_class(cls):
        """Returns contract class via lazy loading."""
        if not cls.CONTRACT_CLASS:
            cls.CONTRACT_CLASS = ContractClass.load(load_nearby_contract("ERC20_Mintable_OZ_0.2.0"))
        return cls.CONTRACT_CLASS

    async def deploy(self):
        """Deploy token contract for charging fees."""
        starknet = self.starknet_wrapper.starknet
        contract_class = FeeToken.get_contract_class()

        fee_token_carried_state = starknet.state.state.contract_states[FeeToken.ADDRESS]
        fee_token_state = fee_token_carried_state.state
        assert not fee_token_state.initialized

        starknet.state.state.contract_definitions[FeeToken.HASH_BYTES] = contract_class
        newly_deployed_fee_token_state = await ContractState.create(
            contract_hash=FeeToken.HASH_BYTES,
            storage_commitment_tree=fee_token_state.storage_commitment_tree
        )

        starknet.state.state.contract_states[FeeToken.ADDRESS] = ContractCarriedState(
            state=newly_deployed_fee_token_state,
            storage_updates={
                # Running the constructor doesn't need to be simulated
                get_selector_from_name("ERC20_name"): StorageLeaf(int.from_bytes(bytes(FeeToken.NAME, "ascii"), "big")),
                get_selector_from_name("ERC20_symbol"): StorageLeaf(int.from_bytes(bytes(FeeToken.SYMBOL, "ascii"), "big")),
                get_selector_from_name("ERC20_decimals"): StorageLeaf(18)
            }
        )

        self.contract = StarknetContract(
            state=starknet.state,
            abi=FeeToken.get_contract_class().abi,
            contract_address=FeeToken.ADDRESS,
            deploy_execution_info=None
        )

        self.starknet_wrapper.store_contract(FeeToken.ADDRESS, self.contract, contract_class)

    async def get_balance(self, address: int) -> int:
        """Return the balance of the contract under `address`."""
        response = await self.contract.balanceOf(address).call()

        balance = Uint256(
            low=response.result.balance.low,
            high=response.result.balance.high
        ).to_felt()
        return balance

    @classmethod
    def get_mint_transaction(cls, to_address: int, amount: Uint256):
        """Construct a transaction object representing minting request"""
        transaction_data = {
            "entry_point_selector": hex(get_selector_from_name("mint")),
            "calldata": [
                str(to_address),
                str(amount.low),
                str(amount.high),
            ],
            "signature": [],
            "contract_address": hex(cls.ADDRESS)
        }
        return InvokeFunction.load(transaction_data)

    async def mint(self, to_address: int, amount: int, lite: bool):
        """
        Mint `amount` tokens at address `to_address`.
        Returns the `tx_hash` (as hex str) if not `lite`; else returns `None`
        """
        amount_uint256 = Uint256.from_felt(amount)

        tx_hash = None
        if lite:
            await self.contract.mint(
                to_address,
                (amount_uint256.low, amount_uint256.high)
            ).invoke()
        else:
            transaction = self.get_mint_transaction(to_address, amount_uint256)
            _, tx_hash_int, _ = await self.starknet_wrapper.invoke(transaction)
            tx_hash = hex(tx_hash_int)

        return tx_hash
