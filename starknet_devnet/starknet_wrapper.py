"""
This module introduces `StarknetWrapper`, a wrapper class of
starkware.starknet.testing.starknet.Starknet.
"""
from copy import deepcopy
import json
import os
from typing import Dict, List, Set, Tuple, Union

import cloudpickle as pickle
from starkware.starknet.business_logic.transaction.objects import (
    CallInfo,
    InternalInvokeFunction,
    InternalDeclare,
    InternalDeploy,
)
from starkware.starknet.business_logic.state.state import BlockInfo, CachedState
from starkware.starknet.services.api.gateway.transaction import (
    InvokeFunction,
    Deploy,
    Declare,
)
from starkware.starknet.testing.starknet import Starknet
from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.services.api.contract_class import EntryPointType, ContractClass
from starkware.starknet.services.api.feeder_gateway.request_objects import CallFunction
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
)
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.testing.objects import FunctionInvocation
from starkware.starknet.compiler.compile import get_selector_from_name
from starkware.solidity.utils import load_nearby_contract
from starkware.crypto.signature.fast_pedersen_hash import pedersen_hash
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionTrace,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockStateUpdate,
    DeployedContract,
    StateDiff,
    StorageEntry,
)

from starknet_devnet.constants import DUMMY_STATE_ROOT

from .accounts import Accounts
from .blueprints.rpc.structures.types import Felt
from .fee_token import FeeToken
from .general_config import DEFAULT_GENERAL_CONFIG
from .origin import NullOrigin, Origin
from .util import (
    DummyExecutionInfo,
    StarknetDevnetException,
    Uint256,
    enable_pickling,
    get_storage_diffs,
    to_bytes,
    get_all_declared_contracts,
)
from .contract_wrapper import ContractWrapper
from .postman_wrapper import DevnetL1L2
from .transactions import DevnetTransactions, DevnetTransaction
from .contracts import DevnetContracts
from .blocks import DevnetBlocks
from .block_info_generator import BlockInfoGenerator
from .devnet_config import DevnetConfig
from .sequencer_api_utils import InternalInvokeFunctionForSimulate

enable_pickling()

# pylint: disable=too-many-instance-attributes
class StarknetWrapper:
    """
    Wraps a Starknet instance and stores data to be returned by the server:
    contract states, transactions, blocks, storages.
    """

    def __init__(self, config: DevnetConfig):
        self.origin: Origin = NullOrigin()
        """Origin chain that this devnet was forked from."""

        self.block_info_generator = BlockInfoGenerator()
        self.blocks = DevnetBlocks(self.origin, lite=config.lite_mode)
        self.config = config
        self.contracts = DevnetContracts(self.origin)
        self.l1l2 = DevnetL1L2()
        self.transactions = DevnetTransactions(self.origin)
        self.starknet: Starknet = None
        self.__current_cached_state = None
        self.__initialized = False
        self.fee_token = FeeToken(self)
        self.accounts = Accounts(self)

        if config.start_time is not None:
            self.set_block_time(config.start_time)

        self.set_gas_price(config.gas_price)

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
            await self.__deploy_wallet()

            await self.__preserve_current_state(starknet.state.state)
            await self.create_empty_block()
            self.__initialized = True

    async def create_empty_block(self):
        """create empty block"""
        self.__update_block_number()
        state_update = await self.__update_state()
        state = self.get_state()
        return await self.blocks.generate(
            None, state, state_update, is_empty_block=True
        )

    async def __preserve_current_state(self, state: CachedState):
        self.__current_cached_state = deepcopy(state)

    async def __init_starknet(self):
        """
        Create and return underlying Starknet instance
        """
        if not self.starknet:
            self.starknet = await Starknet.empty(general_config=DEFAULT_GENERAL_CONFIG)

        return self.starknet

    def get_state(self):
        """
        Returns the StarknetState of the underlying Starknet instance.
        """
        return self.starknet.state

    async def __update_state(
        self,
        deployed_contracts: List[DeployedContract] = None,
        explicitly_declared_contracts: List[int] = None,
        visited_storage_entries: Set[StorageEntry] = None,
        nonces: Dict[int, int] = None,
    ):
        # defaulting
        deployed_contracts = deployed_contracts or []
        explicitly_declared_contracts = explicitly_declared_contracts or []
        visited_storage_entries = visited_storage_entries or set()

        # state update and preservation
        previous_state = self.__current_cached_state
        assert previous_state is not None
        current_state = self.get_state().state
        current_state.block_info = self.block_info_generator.next_block(
            block_info=current_state.block_info,
            general_config=self.get_state().general_config,
        )
        await self.__preserve_current_state(current_state)

        # calculating diffs
        declared_contracts = await get_all_declared_contracts(
            previous_state, explicitly_declared_contracts, deployed_contracts
        )
        storage_diffs = await get_storage_diffs(
            previous_state, current_state, visited_storage_entries
        )
        state_diff = StateDiff(
            deployed_contracts=deployed_contracts,
            declared_contracts=declared_contracts,
            storage_diffs=storage_diffs,
            nonces=nonces or {},
        )

        return BlockStateUpdate(
            block_hash=None,
            new_root=DUMMY_STATE_ROOT,
            old_root=DUMMY_STATE_ROOT,
            state_diff=state_diff,
        )

    async def store_contract(
        self,
        address: int,
        contract: StarknetContract,
        contract_class: ContractClass,
        tx_hash: int = None,
    ):
        """Store the provided data sa wrapped contract"""
        class_hash_bytes = await self.starknet.state.state.get_class_hash_at(address)
        class_hash = int.from_bytes(class_hash_bytes, "big")
        self.contracts.store(
            address=address,
            class_hash=class_hash,
            contract_wrapper=ContractWrapper(contract, contract_class, tx_hash),
        )

    async def __store_transaction(
        self,
        transaction: DevnetTransaction,
        tx_hash: int,
        state_update: Dict,
        error_message: str = None,
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

            block = await self.blocks.generate(
                transaction,
                state,
                state_update=state_update,
            )
            transaction.set_block(block=block)

        self.transactions.store(tx_hash, transaction)

    def set_config(self, config: DevnetConfig):
        """
        Sets the configuration of the devnet.
        """
        self.config = config
        self.blocks.lite = config.lite_mode

    async def declare(self, declare_transaction: Declare) -> Tuple[int, int]:
        """
        Declares the class specified with `declare_transaction`
        Returns (class_hash, transaction_hash)
        """

        internal_declare: InternalDeclare = InternalDeclare.from_external(
            declare_transaction, self.get_state().general_config
        )
        execution_info = await self.starknet.state.execute_tx(internal_declare)
        class_hash_int = int.from_bytes(internal_declare.class_hash, "big")
        # alpha-goerli allows multiple declarations of the same class

        self.contracts.store_class(class_hash_int, declare_transaction.contract_class)
        await self.get_state().state.set_contract_class(
            internal_declare.class_hash, declare_transaction.contract_class
        )

        tx_hash = internal_declare.hash_value
        transaction = DevnetTransaction(
            internal_tx=internal_declare,
            status=TransactionStatus.ACCEPTED_ON_L2,
            execution_info=execution_info,
            transaction_hash=tx_hash,
        )

        self.__update_block_number()
        state_update = await self.__update_state(
            explicitly_declared_contracts=[class_hash_int]
        )

        await self.__store_transaction(
            transaction=transaction,
            tx_hash=tx_hash,
            state_update=state_update,
            error_message=None,
        )

        return class_hash_int, tx_hash

    def __update_block_number(self):
        """Updates just the block number. Returns the old block info to allow reverting"""
        current_cached_state = self.get_state().state
        block_info = current_cached_state.block_info
        current_cached_state.block_info = BlockInfo(
            gas_price=block_info.gas_price,
            block_number=block_info.block_number + 1,
            block_timestamp=block_info.block_timestamp,
            sequencer_address=block_info.sequencer_address,
            starknet_version=block_info.starknet_version,
        )
        return block_info

    # pylint: disable=too-many-locals
    async def deploy(self, deploy_transaction: Deploy) -> Tuple[int, int]:
        """
        Deploys the contract specified with `deploy_transaction`.
        Returns (contract_address, transaction_hash).
        """

        state = self.get_state()
        contract_class = deploy_transaction.contract_definition
        deployed_contracts: List[DeployedContract] = []
        internal_tx: InternalDeploy = InternalDeploy.from_external(
            deploy_transaction, state.general_config
        )
        contract_address = internal_tx.contract_address

        if self.contracts.is_deployed(contract_address):
            tx_hash = self.contracts.get_by_address(contract_address).deployment_tx_hash
            return contract_address, tx_hash

        tx_hash = internal_tx.hash_value

        try:
            preserved_block_info = self.__update_block_number()

            contract = await self.starknet.deploy(
                contract_class=contract_class,
                constructor_calldata=deploy_transaction.constructor_calldata,
                contract_address_salt=deploy_transaction.contract_address_salt,
            )
            execution_info = contract.deploy_call_info
            error_message = None
            status = TransactionStatus.ACCEPTED_ON_L2

            await self.store_contract(
                contract.contract_address, contract, contract_class, tx_hash
            )

            class_hash_bytes = await self.starknet.state.state.get_class_hash_at(
                contract_address
            )
            class_hash_int = int.from_bytes(class_hash_bytes, "big")
            deployed_contracts.append(
                DeployedContract(
                    address=contract.contract_address, class_hash=class_hash_int
                )
            )

            await self.__register_new_contracts(
                execution_info.call_info.internal_calls, tx_hash, deployed_contracts
            )
            state_update = await self.__update_state(
                deployed_contracts=deployed_contracts
            )
        except StarkException as err:
            error_message = err.message
            status = TransactionStatus.REJECTED
            execution_info = DummyExecutionInfo()
            state_update = None

            # restore block info
            self.get_state().state.block_info = preserved_block_info

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
            tx_hash=tx_hash,
        )

        return contract_address, tx_hash

    async def invoke(self, invoke_function: InvokeFunction):
        """Perform invoke according to specifications in `transaction`."""
        state = self.get_state()
        invoke_transaction: InternalInvokeFunction = (
            InternalInvokeFunction.from_external(invoke_function, state.general_config)
        )
        tx_hash = invoke_transaction.hash_value

        try:
            preserved_block_info = self.__update_block_number()

            execution_info = await state.execute_tx(invoke_transaction)
            status = TransactionStatus.ACCEPTED_ON_L2
            error_message = None
            deployed_contracts: List[DeployedContract] = []
            await self.__register_new_contracts(
                execution_info.call_info.internal_calls, tx_hash, deployed_contracts
            )

            state_update = await self.__update_state(
                deployed_contracts=deployed_contracts,
                visited_storage_entries=execution_info.get_visited_storage_entries(),
            )
        except StarkException as err:
            error_message = err.message
            status = TransactionStatus.REJECTED
            execution_info = DummyExecutionInfo()
            state_update = None

            # restore block info
            self.get_state().state.block_info = preserved_block_info

        transaction = DevnetTransaction(invoke_transaction, status, execution_info)

        await self.__store_transaction(
            transaction=transaction,
            state_update=state_update,
            error_message=error_message,
            tx_hash=tx_hash,
        )

        return invoke_function.contract_address, tx_hash

    async def call(self, transaction: CallFunction):
        """Perform call according to specifications in `transaction`."""
        contract_wrapper = self.contracts.get_by_address(transaction.contract_address)

        adapted_result = await contract_wrapper.call(
            entry_point_selector=transaction.entry_point_selector,
            calldata=transaction.calldata,
            caller_address=0,
        )

        return {"result": adapted_result}

    async def __register_new_contracts(
        self,
        internal_calls: List[Union[FunctionInvocation, CallInfo]],
        tx_hash: int,
        deployed_contracts: List[DeployedContract],
    ):
        for internal_call in internal_calls:
            if internal_call.entry_point_type == EntryPointType.CONSTRUCTOR:
                state = self.get_state()
                class_hash_bytes = to_bytes(internal_call.class_hash)
                class_hash_int = int.from_bytes(class_hash_bytes, "big")
                contract_class = await state.state.get_contract_class(class_hash_bytes)

                contract = StarknetContract(
                    state, contract_class.abi, internal_call.contract_address, None
                )
                await self.store_contract(
                    internal_call.contract_address, contract, contract_class, tx_hash
                )
                deployed_contracts.append(
                    DeployedContract(
                        address=contract.contract_address, class_hash=class_hash_int
                    )
                )
            await self.__register_new_contracts(
                internal_call.internal_calls, tx_hash, deployed_contracts
            )

    async def get_storage_at(self, contract_address: int, key: int) -> Felt:
        """
        Returns the storage identified by `key`
        from the contract at `contract_address`.
        """
        state = self.get_state().state
        if self.contracts.is_deployed(contract_address):
            return hex(await state.get_storage_at(contract_address, key))
        return self.origin.get_storage_at(contract_address, key)

    async def load_messaging_contract_in_l1(
        self, network_url: str, contract_address: str, network_id: str
    ) -> dict:
        """Loads the messaging contract at `contract_address`"""
        return self.l1l2.load_l1_messaging_contract(
            self.starknet, network_url, contract_address, network_id
        )

    async def postman_flush(self) -> dict:
        """Handles all pending L1 <> L2 messages and sends them to the other layer."""

        state = self.get_state()
        return await self.l1l2.flush(state)

    async def calculate_trace_and_fee(self, external_tx: InvokeFunction):
        """Calculates trace and fee by simulating tx on state copy."""
        state = self.get_state()

        try:
            internal_tx = InternalInvokeFunctionForSimulate.from_external(
                external_tx, state.general_config
            )
        except AssertionError as error:
            raise StarknetDevnetException(
                status_code=400, message="Invalid format of fee estimation request"
            ) from error

        execution_info = await internal_tx.apply_state_updates(
            # pylint: disable=protected-access
            state.state._copy(),
            state.general_config,
        )

        trace = TransactionTrace(
            validate_invocation=FunctionInvocation.from_optional_internal(
                execution_info.validate_info
            ),
            function_invocation=FunctionInvocation.from_optional_internal(
                execution_info.call_info
            ),
            fee_transfer_invocation=FunctionInvocation.from_optional_internal(
                execution_info.fee_transfer_info
            ),
            signature=external_tx.signature,
        )

        tx_fee = execution_info.actual_fee

        gas_price = state.state.block_info.gas_price
        gas_usage = tx_fee // gas_price if gas_price else 0

        return trace, {
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

    async def get_nonce(self, contract_address: int):
        """Returns nonce of contract with `contract_address`"""
        return await self.get_state().state.get_nonce_at(contract_address)

    async def __deploy_wallet(self):
        """Deploys Starknet CLI's wallet"""
        balance = self.config.initial_balance
        balance_uint256 = Uint256.from_felt(balance)

        artifact_path = os.path.join(
            os.path.dirname(__file__),
            "accounts_artifacts/starknet_cli_wallet/starknet_open_zeppelin_accounts.json",
        )
        with open(artifact_path, encoding="utf-8") as deployment_file:
            deployment_info = json.load(deployment_file)["alpha-goerli"]["__default__"]
        public_key = int(deployment_info["public_key"], 16)

        class_hash = 0x05079DC27D18918EC7A81BE5933620BA90D2191092D70B07110991F7D724920D
        class_hash_bytes = to_bytes(class_hash)
        class_loaded = ContractClass.load(
            load_nearby_contract("accounts_artifacts/starknet_cli_wallet/account")
        )
        await self.starknet.state.state.set_contract_class(
            class_hash_bytes, class_loaded
        )

        wallet_address = int(deployment_info["address"], 16)
        await self.starknet.state.state.deploy_contract(
            wallet_address, class_hash_bytes
        )
        await self.starknet.state.state.set_storage_at(
            wallet_address, get_selector_from_name("public_key"), public_key
        )

        fee_token_address = self.starknet.state.general_config.fee_token_address
        balance_address = pedersen_hash(
            get_selector_from_name("ERC20_balances"), wallet_address
        )
        await self.starknet.state.state.set_storage_at(
            fee_token_address, balance_address, balance_uint256.low
        )
        await self.starknet.state.state.set_storage_at(
            fee_token_address, balance_address + 1, balance_uint256.high
        )

        contract = StarknetContract(
            state=self.starknet.state,
            abi=class_loaded.abi,
            contract_address=wallet_address,
            deploy_call_info=None,
        )

        await self.store_contract(wallet_address, contract, class_loaded)
