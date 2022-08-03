"""
This module introduces `StarknetWrapper`, a wrapper class of
starkware.starknet.testing.starknet.Starknet.
"""
import dataclasses
from copy import deepcopy
from typing import Dict, List, Tuple, Union

import cloudpickle as pickle
from starkware.starknet.business_logic.internal_transaction import (
    InternalInvokeFunction,
    InternalDeclare,
    InternalDeploy,
)
from starkware.starknet.business_logic.internal_transaction import CallInfo
from starkware.starknet.business_logic.state.state import CarriedState
from starkware.starknet.services.api.gateway.transaction import InvokeFunction, Deploy, Declare
from starkware.starknet.testing.starknet import Starknet
from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.business_logic.transaction_fee import calculate_tx_fee
from starkware.starknet.services.api.contract_class import EntryPointType, ContractClass
from starkware.starknet.services.api.feeder_gateway.response_objects import TransactionStatus
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.testing.objects import FunctionInvocation

from .accounts import Accounts
from .fee_token import FeeToken
from .general_config import DEFAULT_GENERAL_CONFIG
from .origin import NullOrigin, Origin
from .util import (
    DummyExecutionInfo,
    enable_pickling,
    generate_state_update,
    to_bytes
)
from .contract_wrapper import ContractWrapper
from .postman_wrapper import DevnetL1L2
from .transactions import DevnetTransactions, DevnetTransaction
from .contracts import DevnetContracts
from .blocks import DevnetBlocks
from .block_info_generator import BlockInfoGenerator

enable_pickling()

@dataclasses.dataclass
class DevnetConfig:
    """Configuration for the devnet."""
    lite_mode_block_hash: bool = False
    lite_mode_deploy_hash: bool = False

#pylint: disable=too-many-instance-attributes
class StarknetWrapper:
    """
    Wraps a Starknet instance and stores data to be returned by the server:
    contract states, transactions, blocks, storages.
    """

    def __init__(self, config: DevnetConfig):
        self.origin: Origin = NullOrigin()
        """Origin chain that this devnet was forked from."""

        self.block_info_generator = BlockInfoGenerator()
        self.blocks = DevnetBlocks(self.origin, lite=config.lite_mode_block_hash)
        self.config = config
        self.contracts = DevnetContracts(self.origin)
        self.l1l2 = DevnetL1L2()
        self.transactions = DevnetTransactions(self.origin)
        self.starknet: Starknet = None
        self.__current_carried_state = None
        self.__initialized = False
        self.fee_token = FeeToken(self)
        self.accounts = Accounts(self)

    @staticmethod
    def load(path: str) -> "StarknetWrapper":
        """Load a serialized instance of this class from `path`."""
        with open(path, "rb") as file:
            return pickle.load(file)

    async def initialize(self):
        """Initialize the underlying starknet instance, fee_token and accounts."""
        if not self.__initialized:
            starknet = await self.__init_starknet()

            await self.fee_token.deploy()
            await self.accounts.deploy()

            await self.__preserve_current_state(starknet.state.state)
            await self.create_empty_block()
            self.__initialized = True

    async def create_empty_block(self):
        """create empty block"""
        state_update = await self.__update_state()
        state = self.get_state()
        state_root = self.__get_state_root()
        return self.blocks.generate_empty(state, state_root, state_update)

    async def __preserve_current_state(self, state: CarriedState):
        self.__current_carried_state = deepcopy(state)
        self.__current_carried_state.shared_state = state.shared_state

    async def __init_starknet(self):
        """
        Create and return underlying Starknet instance
        """
        if not self.starknet:
            self.starknet = await Starknet.empty(general_config=DEFAULT_GENERAL_CONFIG)

        return self.starknet

    def get_state(self):
        """
        Returns the StarknetState of the underlyling Starknet instance.
        """
        return self.starknet.state

    async def __update_state(self):
        previous_state = self.__current_carried_state
        assert previous_state is not None
        current_carried_state = self.get_state().state
        state = self.get_state()

        current_carried_state.block_info = self.block_info_generator.next_block(
            block_info=current_carried_state.block_info,
            general_config=state.general_config
        )

        if not self.config.lite_mode_block_hash:
            # This is the most time-intensive part of the function.
            # With only skipping it in lite-mode, we still get the time benefit.
            # In regular mode it's needed for state update calculation and block state_root calculation.
            updated_shared_state = await current_carried_state.shared_state.apply_state_updates(
                ffc=current_carried_state.ffc,
                previous_carried_state=previous_state,
                current_carried_state=current_carried_state
            )

            state.state.shared_state = updated_shared_state
            await self.__preserve_current_state(state.state)

            return generate_state_update(previous_state, current_carried_state)

        return None

    def __get_state_root(self):
        return self.get_state().state.shared_state.contract_states.root

    def store_contract(self,
        address: int, contract: StarknetContract, contract_class: ContractClass, tx_hash: int = None):
        """Store the provided data sa wrapped contract"""
        self.contracts.store(address, ContractWrapper(contract, contract_class, tx_hash))

    async def __store_transaction(
        self, transaction: DevnetTransaction, tx_hash: int,
        state_update: Dict, error_message: str=None
    ) -> None:
        """
        Stores the provided data as a deploy transaction in `self.transactions`.
        Generates a new block
        """

        if transaction.status == TransactionStatus.REJECTED:
            assert error_message, "error_message must be present if tx rejected"
            transaction.set_failure_reason(error_message)
        else:
            state = self.get_state()
            state_root = self.__get_state_root()

            block = await self.blocks.generate(
                transaction,
                state,
                state_root,
                state_update=state_update,
            )
            transaction.set_block(block=block)

        self.transactions.store(tx_hash, transaction)

    def set_config(self, config: DevnetConfig):
        """
        Sets the configuration of the devnet.
        """
        self.config = config
        self.blocks.lite = config.lite_mode_block_hash

    async def declare(self, declare_transaction: Declare) -> Tuple[int, int]:
        """
        Declares the class specified with `declare_transaction`
        Returns (class_hash, transaction_hash)
        """

        internal_declare: InternalDeclare = InternalDeclare.from_external(
            declare_transaction,
            self.starknet.state.general_config
        )
        declared_class = await self.starknet.declare(
            contract_class=declare_transaction.contract_class,
        )
        self.contracts.store_class(declared_class.class_hash, declare_transaction.contract_class)

        tx_hash = internal_declare.hash_value
        transaction = DevnetTransaction(
            internal_tx=internal_declare,
            status=TransactionStatus.ACCEPTED_ON_L2,
            execution_info=DummyExecutionInfo(),
            transaction_hash=tx_hash
        )

        state_update = await self.__update_state()

        await self.__store_transaction(
            transaction=transaction,
            tx_hash=tx_hash,
            state_update=state_update,
            error_message=None
        )

        return declared_class.class_hash, tx_hash

    async def deploy(self, deploy_transaction: Deploy) -> Tuple[int, int]:
        """
        Deploys the contract specified with `deploy_transaction`.
        Returns (contract_address, transaction_hash).
        """

        state = self.get_state()
        contract_class = deploy_transaction.contract_definition

        internal_tx: InternalDeploy = InternalDeploy.from_external(deploy_transaction, state.general_config)
        contract_address = internal_tx.contract_address

        if self.contracts.is_deployed(contract_address):
            tx_hash = self.contracts.get_by_address(contract_address).deployment_tx_hash
            return contract_address, tx_hash

        if self.config.lite_mode_deploy_hash:
            tx_hash = self.transactions.get_count()
        else:
            tx_hash = internal_tx.hash_value

        try:
            contract = await self.starknet.deploy(
                contract_class=contract_class,
                constructor_calldata=deploy_transaction.constructor_calldata,
                contract_address_salt=deploy_transaction.contract_address_salt
            )
            execution_info = contract.deploy_execution_info
            error_message = None
            status = TransactionStatus.ACCEPTED_ON_L2

            self.store_contract(contract.contract_address, contract, contract_class, tx_hash)
            state_update = await self.__update_state()
        except StarkException as err:
            error_message = err.message
            status = TransactionStatus.REJECTED
            execution_info = DummyExecutionInfo()
            state_update = None

        transaction = DevnetTransaction(
            internal_tx=internal_tx,
            status=status,
            execution_info=execution_info,
            transaction_hash=tx_hash,
        )

        await self.__store_transaction(
            transaction=transaction,
            state_update=state_update,
            error_message=error_message,
            tx_hash=tx_hash
        )

        await self.__register_new_contracts(execution_info.call_info.internal_calls, tx_hash)

        return contract_address, tx_hash

    async def invoke(self, invoke_function: InvokeFunction):
        """Perform invoke according to specifications in `transaction`."""
        state = self.get_state()
        invoke_transaction: InternalInvokeFunction = InternalInvokeFunction.from_external(invoke_function, state.general_config)

        try:
            contract_wrapper = self.contracts.get_by_address(invoke_transaction.contract_address)
            adapted_result, execution_info = await contract_wrapper.invoke(
                entry_point_selector=invoke_transaction.entry_point_selector,
                calldata=invoke_transaction.calldata,
                signature=invoke_transaction.signature,
                caller_address=invoke_transaction.caller_address,
                max_fee=invoke_transaction.max_fee
            )
            status = TransactionStatus.ACCEPTED_ON_L2
            error_message = None
            state_update = await self.__update_state()
        except StarkException as err:
            error_message = err.message
            status = TransactionStatus.REJECTED
            execution_info = DummyExecutionInfo()
            adapted_result = []
            state_update = None

        transaction = DevnetTransaction(invoke_transaction, status, execution_info)
        tx_hash = transaction.transaction_hash

        await self.__store_transaction(
            transaction=transaction,
            state_update=state_update,
            error_message=error_message,
            tx_hash=tx_hash
        )

        await self.__register_new_contracts(execution_info.call_info.internal_calls, tx_hash)

        return invoke_function.contract_address, tx_hash, { "result": adapted_result }

    async def call(self, transaction: InvokeFunction):
        """Perform call according to specifications in `transaction`."""
        contract_wrapper = self.contracts.get_by_address(transaction.contract_address)

        adapted_result = await contract_wrapper.call(
            entry_point_selector=transaction.entry_point_selector,
            calldata=transaction.calldata,
            signature=transaction.signature,
            caller_address=0,
            max_fee=transaction.max_fee
        )

        return { "result": adapted_result }

    async def __register_new_contracts(self, internal_calls: List[Union[FunctionInvocation, CallInfo]], tx_hash: int):
        for internal_call in internal_calls:
            if internal_call.entry_point_type == EntryPointType.CONSTRUCTOR:
                state = self.get_state()
                class_hash = to_bytes(internal_call.class_hash)
                contract_class = state.state.get_contract_class(class_hash)

                contract = StarknetContract(state, contract_class.abi, internal_call.contract_address, None)
                self.store_contract(internal_call.contract_address, contract, contract_class, tx_hash)
            await self.__register_new_contracts(internal_call.internal_calls, tx_hash)

    async def get_storage_at(self, contract_address: int, key: int) -> str:
        """
        Returns the storage identified by `key`
        from the contract at `contract_address`.
        """
        state = self.get_state()
        contract_states = state.state.contract_states

        contract_state = contract_states[contract_address]
        if key in contract_state.storage_updates:
            return hex(contract_state.storage_updates[key].value)
        return self.origin.get_storage_at(contract_address, key)

    async def load_messaging_contract_in_l1(self, network_url: str, contract_address: str, network_id: str) -> dict:
        """Loads the messaging contract at `contract_address`"""
        return self.l1l2.load_l1_messaging_contract(self.starknet, network_url, contract_address, network_id)

    async def postman_flush(self) -> dict:
        """Handles all pending L1 <> L2 messages and sends them to the other layer. """

        state = self.get_state()
        return await self.l1l2.flush(state)

    async def calculate_actual_fee(self, external_tx: InvokeFunction):
        """Calculates actual fee"""
        state = self.get_state()
        internal_tx = InternalInvokeFunction.from_external_query_tx(external_tx, state.general_config)

        child_state = state.state.create_child_state_for_querying()
        call_info = await internal_tx.execute(child_state, state.general_config, only_query=True)

        tx_fee = calculate_tx_fee(
            state=child_state,
            call_info=call_info,
            general_config=state.general_config
        )

        gas_price = state.state.block_info.gas_price
        gas_usage = tx_fee // gas_price if gas_price else 0

        return {
            "overall_fee": tx_fee,
            "unit": "wei",
            "gas_price": gas_price,
            "gas_usage": gas_usage,
        }

    def increase_block_time(self, time_s: int):
        """Increases the block time by `time_s`."""
        self.block_info_generator.increase_time(time_s)

    def set_block_time(self, time_s: int):
        """Sets the block time to `time_s`."""
        self.block_info_generator.set_next_block_time(time_s)

    def set_gas_price(self, gas_price: int):
        """Sets gas price to `gas_price`."""
        self.block_info_generator.set_gas_price(gas_price)
