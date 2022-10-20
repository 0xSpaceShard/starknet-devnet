"""
This module introduces `StarknetWrapper`, a wrapper class of
starkware.starknet.testing.starknet.Starknet.
"""
from copy import deepcopy
from types import TracebackType
from typing import Dict, List, Optional, Set, Tuple, Type, Union

import cloudpickle as pickle
from starkware.python.utils import as_non_optional
from starkware.starknet.business_logic.transaction.fee import calculate_tx_fee
from starkware.starknet.business_logic.transaction.objects import (
    CallInfo,
    InternalInvokeFunction,
    InternalDeclare,
    InternalDeploy,
    InternalDeployAccount,
    InternalL1Handler,
    InternalTransaction,
    TransactionExecutionInfo,
)
from starkware.starknet.business_logic.state.state import BlockInfo, CachedState
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.services.api.gateway.transaction import (
    InvokeFunction,
    Deploy,
    DeployAccount,
    Declare,
)
from starkware.starknet.testing.contract_utils import get_abi
from starkware.starknet.testing.starknet import Starknet
from starkware.starkware_utils.error_handling import StarkException
from starkware.starknet.services.api.contract_class import EntryPointType, ContractClass
from starkware.starknet.services.api.feeder_gateway.request_objects import (
    CallFunction,
    CallL1Handler,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionStatus,
)
from starkware.starknet.testing.contract import StarknetContract
from starkware.starknet.testing.objects import FunctionInvocation, StarknetCallInfo
from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    TransactionTrace,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    BlockStateUpdate,
    DeployedContract,
    StateDiff,
    StorageEntry,
)
from starkware.starkware_utils.error_handling import StarkErrorCode

from starknet_devnet.util import to_bytes, get_fee_estimation_info
from starknet_devnet.constants import DUMMY_STATE_ROOT, OZ_ACCOUNT_CLASS_HASH

from .lite_mode.lite_internal_deploy import LiteInternalDeploy
from .lite_mode.lite_starknet import LiteStarknet

from .accounts import Accounts
from .blueprints.rpc.structures.types import Felt
from .fee_token import FeeToken
from .general_config import DEFAULT_GENERAL_CONFIG
from .origin import NullOrigin, Origin
from .util import (
    StarknetDevnetException,
    enable_pickling,
    get_storage_diffs,
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
            await self.__predeclare_oz_account()

            await self.__preserve_current_state(starknet.state.state)
            await self.create_empty_block()
            self.__initialized = True

    async def create_empty_block(self):
        """create empty block"""
        self._update_block_number()
        state_update = await self._update_state()
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

    async def _update_state(
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

    async def _store_transaction(
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

    async def declare(self, external_tx: Declare) -> Tuple[int, int]:
        """
        Declares the class specified with `declare_transaction`
        Returns (class_hash, transaction_hash)
        """

        async with self.__get_transaction_handler() as tx_handler:
            tx_handler.internal_tx = InternalDeclare.from_external(
                external_tx, self.get_state().general_config
            )
            # calculate class hash here if execution fails
            class_hash_int = int.from_bytes(tx_handler.internal_tx.class_hash, "big")

            tx_handler.execution_info = await self.starknet.state.execute_tx(
                tx_handler.internal_tx
            )

            tx_handler.explicitly_declared.append(class_hash_int)

            # alpha-goerli allows multiple declarations of the same class
            self.contracts.store_class(class_hash_int, external_tx.contract_class)
            await self.get_state().state.set_contract_class(
                tx_handler.internal_tx.class_hash, external_tx.contract_class
            )

        return class_hash_int, tx_handler.internal_tx.hash_value

    def _update_block_number(self):
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

    def __get_transaction_handler(self):
        class TransactionHandler:
            """Class for with-blocks in transactions"""

            internal_tx: InternalTransaction
            execution_info: TransactionExecutionInfo
            internal_calls: List[CallInfo] = []
            deployed_contracts: List[DeployedContract] = []
            explicitly_declared: List[int] = []
            visited_storage_entries: Set[StorageEntry] = set()

            def __init__(self, starknet_wrapper: StarknetWrapper):
                self.starknet_wrapper = starknet_wrapper
                self.preserved_block_info = starknet_wrapper._update_block_number()

            async def __aenter__(self):
                return self

            async def __aexit__(
                self,
                exc_type: Optional[Type[BaseException]],
                exc: Optional[BaseException],
                traceback: Optional[TracebackType],
            ):
                assert self.internal_tx is not None
                tx_hash = self.internal_tx.hash_value

                if exc_type:
                    assert isinstance(exc, StarkException)
                    error_message = exc.message
                    status = TransactionStatus.REJECTED
                    self.execution_info = TransactionExecutionInfo.empty()
                    state_update = None

                    # restore block info
                    self.starknet_wrapper.get_state().state.block_info = (
                        self.preserved_block_info
                    )
                else:
                    error_message = None
                    status = TransactionStatus.ACCEPTED_ON_L2

                    assert self.execution_info is not None
                    if self.execution_info.call_info:
                        await self.starknet_wrapper._register_new_contracts(
                            self.internal_calls,
                            tx_hash,
                            self.deployed_contracts,
                        )

                    state_update = await self.starknet_wrapper._update_state(
                        deployed_contracts=self.deployed_contracts,
                        visited_storage_entries=self.visited_storage_entries,
                        explicitly_declared_contracts=self.explicitly_declared,
                    )

                transaction = DevnetTransaction(
                    internal_tx=self.internal_tx,
                    status=status,
                    execution_info=self.execution_info,
                    transaction_hash=tx_hash,
                )

                await self.starknet_wrapper._store_transaction(
                    transaction=transaction,
                    state_update=state_update,
                    error_message=error_message,
                    tx_hash=tx_hash,
                )

                return True

        return TransactionHandler(self)

    async def deploy_account(self, external_tx: DeployAccount):
        """Deploys account and returns (address, tx_hash)"""

        state = self.get_state()
        async with self.__get_transaction_handler() as tx_handler:
            tx_handler.internal_tx = InternalDeployAccount.from_external(
                external_tx, state.general_config
            )
            tx_handler.execution_info = await state.execute_tx(tx_handler.internal_tx)
            tx_handler.internal_calls = (
                tx_handler.execution_info.call_info.internal_calls
            )

        account_address = calculate_contract_address_from_hash(
            salt=external_tx.contract_address_salt,
            class_hash=external_tx.class_hash,
            constructor_calldata=external_tx.constructor_calldata,
            deployer_address=0,
        )

        return (
            account_address,
            tx_handler.internal_tx.hash_value,
        )

    async def deploy(self, deploy_transaction: Deploy) -> Tuple[int, int]:
        """
        Deploys the contract specified with `deploy_transaction`.
        Returns (contract_address, transaction_hash).
        """

        transactions_count = self.transactions.get_count()
        contract_class = deploy_transaction.contract_definition

        if self.config.lite_mode:
            internal_tx: LiteInternalDeploy = LiteInternalDeploy.from_external(
                deploy_transaction, tx_number=transactions_count
            )
        else:
            internal_tx: InternalDeploy = InternalDeploy.from_external(
                deploy_transaction, self.get_state().general_config
            )

        contract_address = internal_tx.contract_address

        if self.contracts.is_deployed(contract_address):
            tx_hash = self.contracts.get_by_address(contract_address).deployment_tx_hash
            return contract_address, tx_hash

        tx_hash = internal_tx.hash_value

        async with self.__get_transaction_handler() as tx_handler:
            tx_handler.internal_tx = internal_tx

            if self.config.lite_mode:
                contract = await LiteStarknet.deploy(
                    self,
                    contract_class=contract_class,
                    constructor_calldata=deploy_transaction.constructor_calldata,
                    contract_address_salt=deploy_transaction.contract_address_salt,
                    starknet=self.starknet,
                    tx_number=transactions_count,
                )
            else:
                contract = await self.__deploy(internal_tx, contract_class)

            tx_handler.execution_info = contract.deploy_call_info
            tx_handler.internal_calls = (
                contract.deploy_call_info.call_info.internal_calls
            )

            await self.store_contract(
                contract.contract_address, contract, contract_class, tx_hash
            )

            class_hash_bytes = await self.starknet.state.state.get_class_hash_at(
                contract_address
            )
            class_hash_int = int.from_bytes(class_hash_bytes, "big")
            tx_handler.deployed_contracts.append(
                DeployedContract(
                    address=contract.contract_address, class_hash=class_hash_int
                )
            )

        return contract_address, tx_hash

    async def invoke(self, external_tx: InvokeFunction):
        """Perform invoke according to specifications in `transaction`."""
        state = self.get_state()

        async with self.__get_transaction_handler() as tx_handler:
            tx_handler.internal_tx = InternalInvokeFunction.from_external(
                external_tx, state.general_config
            )
            tx_handler.execution_info = await state.execute_tx(tx_handler.internal_tx)
            tx_handler.internal_calls = (
                tx_handler.execution_info.call_info.internal_calls
            )
            tx_handler.visited_storage_entries = (
                tx_handler.execution_info.get_visited_storage_entries()
            )

        return external_tx.contract_address, tx_handler.internal_tx.hash_value

    async def call(self, transaction: CallFunction):
        """Perform call according to specifications in `transaction`."""
        contract_wrapper = self.contracts.get_by_address(transaction.contract_address)

        adapted_result = await contract_wrapper.call(
            entry_point_selector=transaction.entry_point_selector,
            calldata=transaction.calldata,
            caller_address=0,
        )

        return {"result": adapted_result}

    async def __deploy(self, deploy_tx: InternalDeploy, contract_class: ContractClass):
        """
        Replacement for self.starknet.deploy that allows usage of InternalDeploy right away.
        This way InternalDeploy doesn't have to be created twice, calculating hash every time.
        """
        state = self.get_state()
        await state.state.set_contract_class(
            class_hash=deploy_tx.contract_hash, contract_class=contract_class
        )

        tx_execution_info = await state.execute_tx(tx=deploy_tx)

        deploy_call_info = StarknetCallInfo.from_internal(
            call_info=as_non_optional(tx_execution_info.call_info),
            result=(),
            main_call_events=[],
        )

        return StarknetContract(
            state=state,
            abi=get_abi(contract_class=contract_class),
            contract_address=deploy_tx.contract_address,
            deploy_call_info=deploy_call_info,
        )

    async def _register_new_contracts(
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
            await self._register_new_contracts(
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
                code=StarkErrorCode.MALFORMED_REQUEST,
                status_code=400,
                message="Invalid format of fee estimation request",
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

        fee_estimation_info = get_fee_estimation_info(
            execution_info.actual_fee, state.state.block_info.gas_price
        )

        return trace, fee_estimation_info

    async def estimate_message_fee(self, call: CallL1Handler):
        """Estimate fee of message from L1 to L2"""
        state = self.get_state()
        internal_call: InternalL1Handler = call.to_internal(
            state.general_config.chain_id.value
        )

        execution_info = await internal_call.apply_state_updates(
            # pylint: disable=protected-access
            state.state._copy(),
            state.general_config,
        )

        actual_fee = calculate_tx_fee(
            resources=execution_info.actual_resources,
            gas_price=state.general_config.min_gas_price,
            general_config=state.general_config,
        )

        fee_estimation_info = get_fee_estimation_info(
            actual_fee, state.state.block_info.gas_price
        )
        return fee_estimation_info

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

    async def __predeclare_oz_account(self):
        await self.get_state().state.set_contract_class(
            to_bytes(OZ_ACCOUNT_CLASS_HASH), oz_account_class
        )
