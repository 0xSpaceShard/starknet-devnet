# pylint: disable=too-many-lines
"""
This module introduces `StarknetWrapper`, a wrapper class of
starkware.starknet.testing.starknet.Starknet.
"""
import asyncio
import pprint
from copy import deepcopy
from types import TracebackType
from typing import Dict, List, Optional, Set, Tuple, Type, Union

import cloudpickle as pickle
from starkware.starknet.business_logic.execution.objects import (
    ExecutionResourcesManager,
    ResourcesMapping,
)
from starkware.starknet.business_logic.state.state import (
    BlockInfo,
    CachedState,
    StateSyncifier,
    UpdatesTrackerState,
)
from starkware.starknet.business_logic.state.storage_domain import StorageDomain
from starkware.starknet.business_logic.transaction.fee import calculate_tx_fee
from starkware.starknet.business_logic.transaction.objects import (
    CallInfo,
    InternalAccountTransaction,
    InternalDeploy,
    InternalDeployAccount,
    InternalL1Handler,
    TransactionExecutionInfo,
)
from starkware.starknet.business_logic.utils import calculate_tx_resources
from starkware.starknet.core.os.contract_address.contract_address import (
    calculate_contract_address_from_hash,
)
from starkware.starknet.core.os.contract_class.compiled_class_hash import (
    compute_compiled_class_hash,
)
from starkware.starknet.definitions.error_codes import StarknetErrorCode
from starkware.starknet.definitions.transaction_type import TransactionType
from starkware.starknet.services.api.contract_class.contract_class import (
    CompiledClass,
    ContractClass,
    DeprecatedCompiledClass,
    EntryPointType,
)
from starkware.starknet.services.api.feeder_gateway.request_objects import (
    CallFunction,
    CallL1Handler,
)
from starkware.starknet.services.api.feeder_gateway.response_objects import (
    LATEST_BLOCK_ID,
    PENDING_BLOCK_ID,
    BlockStateUpdate,
    ClassHashPair,
    ContractAddressHashPair,
    ExecutionStatus,
    FinalityStatus,
    StarknetBlock,
    StateDiff,
    StorageEntry,
    TransactionStatus,
    TransactionTrace,
)
from starkware.starknet.services.api.gateway.transaction import (
    AccountTransaction,
    Declare,
    DeployAccount,
    DeprecatedDeclare,
    InvokeFunction,
)
from starkware.starknet.services.api.messages import StarknetMessageToL1
from starkware.starknet.services.utils.sequencer_api_utils import (
    InternalInvokeFunctionForSimulate,
)
from starkware.starknet.testing.objects import FunctionInvocation
from starkware.starknet.testing.starknet import Starknet
from starkware.starknet.third_party.open_zeppelin.starknet_contracts import (
    account_contract as oz_account_class,
)
from starkware.starkware_utils.error_handling import (
    StarkErrorCode,
    StarkException,
    stark_assert_le,
)

from .accounts import Accounts
from .block_info_generator import BlockInfoGenerator
from .blocks import DevnetBlocks
from .blueprints.rpc.structures.types import BlockId, Felt
from .chargeable_account import ChargeableAccount
from .compiler import select_compiler
from .constants import (
    DUMMY_PENDING_BLOCK_HASH,
    DUMMY_STATE_ROOT,
    LEGACY_TX_VERSION,
    STARKNET_CLI_ACCOUNT_CLASS_HASH,
)
from .devnet_config import DevnetConfig
from .fee_token import FeeToken
from .forked_state import get_forked_starknet
from .general_config import build_devnet_general_config
from .origin import ForkedOrigin, NullOrigin
from .postman_wrapper import DevnetL1L2
from .transactions import (
    DevnetTransaction,
    DevnetTransactions,
    create_empty_internal_declare,
    create_empty_internal_deploy,
    create_genesis_block_transaction,
)
from .udc import UDC
from .util import (
    StarknetDevnetException,
    UndeclaredClassDevnetException,
    assert_not_declared,
    assert_recompiled_class_hash,
    enable_pickling,
    get_all_declared_cairo0_classes,
    get_all_declared_cairo1_classes,
    get_fee_estimation_info,
    get_replaced_classes,
    get_storage_diffs,
    group_classes_by_version,
    logger,
    stark_assert_call_succeeded,
    warn,
)

enable_pickling()

DEFAULT_BLOCK_ID = LATEST_BLOCK_ID


# pylint: disable=too-many-instance-attributes
# pylint: disable=too-many-public-methods
# pylint: disable=too-many-locals
class StarknetWrapper:
    """
    Wraps a Starknet instance and stores data to be returned by the server:
    contract states, transactions, blocks, storages.
    """

    def __init__(self, config: DevnetConfig):
        self.origin = (
            ForkedOrigin(config.fork_network, config.fork_block)
            if config.fork_network
            else NullOrigin()
        )
        """Origin chain that this devnet was forked from."""

        self.block_info_generator = BlockInfoGenerator()
        self.blocks = None
        self.config = config
        self.l1l2 = DevnetL1L2()
        self.transactions = DevnetTransactions(self.origin)
        self.starknet: Starknet = None
        self.__current_cached_state = None
        self.__initialized = False
        self.fee_token = FeeToken(self)
        self.accounts = Accounts(self)
        self.__udc = UDC(self)
        self.pending_txs: List[DevnetTransaction] = []
        self.__latest_state = None
        self._contract_classes: Dict[int, Union[DeprecatedCompiledClass, ContractClass]]
        """If v2 - store sierra, otherwise store old class; needed for get_class_by_hash"""
        self.genesis_block_number = None
        self._compiler = select_compiler(config)

        if config.start_time is not None:
            self.set_block_time(config.start_time)

        self.__set_gas_price(config.gas_price)

    @staticmethod
    def load(path: str) -> "StarknetWrapper":
        """Load a serialized instance of this class from `path`."""
        with open(path, "rb") as file:
            return pickle.load(file)

    async def initialize(self):
        """Initialize the underlying starknet instance, fee_token and accounts."""
        if not self.__initialized:
            starknet = await self.__init_starknet()

            # ok that it's here so that e.g. reset includes reset of blocks
            self.blocks = DevnetBlocks(self.origin, lite=self.config.lite_mode)

            self._contract_classes = {}
            await self.fee_token.deploy()
            await self.accounts.deploy()
            await self.__deploy_chargeable_account()
            await self.__predeclare_starknet_cli_account()
            await self.__udc.deploy()

            await self.__preserve_current_state(starknet.state.state)
            await self.__create_genesis_block()
            self.__latest_state = self.get_state().copy()
            self.__initialized = True

    async def __create_genesis_block(self):
        """Create genesis block"""
        transactions: List[DevnetTransaction] = []
        transaction_hash = 1

        self.genesis_block_number = self.blocks.get_next_block_number()

        # Declare transactions
        declare_hashes = [
            FeeToken.HASH,
            UDC.HASH,
            self.config.account_class.hash,
            STARKNET_CLI_ACCOUNT_CLASS_HASH,
        ]
        for class_hash in declare_hashes:
            internal_declare = create_empty_internal_declare(
                transaction_hash, class_hash
            )
            declare_transaction = create_genesis_block_transaction(
                internal_declare,
                TransactionType.DECLARE,
                block_number=self.genesis_block_number,
                transaction_index=len(transactions),
            )
            transactions.append(declare_transaction)
            transaction_hash += 1

        # Deploy transactions
        deploy_data = [
            (FeeToken.HASH, FeeToken.ADDRESS),
            (UDC.HASH, UDC.ADDRESS),
            (self.config.account_class.hash, ChargeableAccount.ADDRESS),
        ]
        for account in self.accounts:
            deploy_data.append((account.class_hash, account.address))

        for class_hash, contract_address in deploy_data:
            # this might be the only place where DEPLOY tx is used
            internal_deploy = create_empty_internal_deploy(
                transaction_hash, class_hash, contract_address
            )
            deploy_transaction = create_genesis_block_transaction(
                internal_deploy,
                TransactionType.DEPLOY,
                block_number=self.genesis_block_number,
                transaction_index=len(transactions),
            )
            transactions.append(deploy_transaction)
            transaction_hash += 1

        self._update_block_number()
        state = self.get_state()
        state_update = await self.update_pending_state()
        await self.blocks.generate_pending(transactions, state, state_update)
        await self.generate_latest_block(block_hash=0)

        for transaction in transactions:
            self.transactions.store(transaction.transaction_hash, transaction)

    async def create_empty_block(self) -> StarknetBlock:
        """Create empty block."""
        self._update_block_number()
        state_update = await self.update_pending_state()
        self.__latest_state = self.get_state().copy()
        return await self.blocks.generate_empty_block(self.get_state(), state_update)

    async def __preserve_current_state(self, state: CachedState):
        self.__current_cached_state = deepcopy(state)

    async def __init_starknet(self):
        """
        Create and return underlying Starknet instance
        """
        if not self.starknet:
            if self.__is_fork():
                print(
                    f"Forking {self.config.fork_network.url} from block {self.config.fork_block}"
                )

                self.starknet = get_forked_starknet(
                    feeder_gateway_client=self.config.fork_network,
                    block_number=self.config.fork_block,
                    gas_price=self.block_info_generator.gas_price,
                    chain_id=self.config.chain_id,
                )
            else:
                self.starknet = await Starknet.empty(
                    general_config=build_devnet_general_config(self.config.chain_id)
                )

        return self.starknet

    def __is_fork(self):
        return bool(self.config.fork_network)

    def get_state(self):
        """
        Returns the StarknetState of the underlying Starknet instance.
        """
        return self.starknet.state

    async def update_pending_state(  # pylint: disable=too-many-arguments
        self,
        deployed_contracts: List[ContractAddressHashPair] = None,
        explicitly_declared_old: List[int] = None,
        explicitly_declared: List[ClassHashPair] = None,
        visited_storage_entries: Set[StorageEntry] = None,
        nonces: Dict[int, int] = None,
    ):
        """Update pending state."""
        # defaulting
        deployed_contracts = deployed_contracts or []
        explicitly_declared_old = explicitly_declared_old or []
        explicitly_declared = explicitly_declared or []
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

        (
            deployed_cairo0_contracts,
            deployed_cairo1_contracts,
        ) = await group_classes_by_version(deployed_contracts, current_state)

        # calculating diffs
        old_declared_contracts = await get_all_declared_cairo0_classes(
            previous_state, explicitly_declared_old, deployed_cairo0_contracts
        )
        declared_classes = await get_all_declared_cairo1_classes(
            previous_state, explicitly_declared, deployed_cairo1_contracts
        )
        replaced = await get_replaced_classes(previous_state, current_state)
        storage_diffs = await get_storage_diffs(
            previous_state, current_state, visited_storage_entries
        )
        state_diff = StateDiff(
            deployed_contracts=deployed_contracts,
            old_declared_contracts=old_declared_contracts,
            declared_classes=declared_classes,
            replaced_classes=replaced,
            storage_diffs=storage_diffs,
            nonces=nonces or {},
        )

        return BlockStateUpdate(
            block_hash=DUMMY_PENDING_BLOCK_HASH,
            new_root=DUMMY_STATE_ROOT,
            old_root=DUMMY_STATE_ROOT,
            state_diff=state_diff,
        )

    async def declare(
        self, external_tx: Union[Declare, DeprecatedDeclare]
    ) -> Tuple[int, int]:
        """
        Declares the class specified with `declare_transaction`
        Returns (class_hash, transaction_hash)
        """

        state = self.get_state()
        async with self.__get_transaction_handler(external_tx) as tx_handler:
            # extract class hash here if execution later fails
            class_hash = tx_handler.internal_tx.class_hash

            # this is done now (before execute_tx) so that later we can assert it hasn't been deployed
            compiled_class_hash = await state.state.get_compiled_class_hash(class_hash)

            # check if Cairo 1 / Declare v2
            if isinstance(external_tx, Declare):
                await assert_not_declared(class_hash, compiled_class_hash)
                compiled_class_hash = tx_handler.internal_tx.compiled_class_hash
                compiled_class = self._compiler.compile_contract_class(
                    external_tx.contract_class
                )
                compiled_class_hash_computed = compute_compiled_class_hash(
                    compiled_class
                )
                assert_recompiled_class_hash(
                    compiled_class_hash_computed, compiled_class_hash
                )

                # Even though execute_tx is performed, class needs to be set explicitly
                tx_handler.execution_info = await state.execute_tx(
                    tx_handler.internal_tx
                )

                await state.state.set_compiled_class_hash(
                    class_hash=class_hash, compiled_class_hash=compiled_class_hash
                )
                tx_handler.explicitly_declared.append(
                    ClassHashPair(class_hash, compiled_class_hash)
                )

            else:  # Cairo 0.x / Declare v1
                tx_handler.execution_info = await state.execute_tx(
                    tx_handler.internal_tx
                )
                compiled_class_hash = class_hash
                compiled_class = external_tx.contract_class
                tx_handler.explicitly_declared_old.append(class_hash)

            state.state.compiled_classes[compiled_class_hash] = compiled_class
            self._contract_classes[class_hash] = external_tx.contract_class

        return class_hash, tx_handler.internal_tx.hash_value

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

    def __get_transaction_handler(  # pylint: disable=too-many-statements
        self, external_tx: Optional[AccountTransaction] = None
    ):
        class TransactionHandler:
            """Class for with-blocks in transactions"""

            internal_tx: Optional[InternalAccountTransaction] = None
            execution_info: TransactionExecutionInfo = TransactionExecutionInfo.empty()
            internal_calls: List[CallInfo] = []
            deployed_contracts: List[ContractAddressHashPair] = []
            explicitly_declared_old: List[int] = []
            explicitly_declared: List[ClassHashPair] = []
            visited_storage_entries: Set[StorageEntry] = set()

            def __init__(self, starknet_wrapper: StarknetWrapper):
                self.starknet_wrapper = starknet_wrapper
                self.preserved_block_info = starknet_wrapper._update_block_number()
                if external_tx:
                    self._validate_fee(external_tx)
                    self.internal_tx = InternalAccountTransaction.from_external(
                        external_tx, starknet_wrapper.get_state().general_config
                    )

            def _check_nonce(self, state: UpdatesTrackerState):
                nonce = state.get_nonce_at(
                    storage_domain=StorageDomain.ON_CHAIN,
                    contract_address=self.internal_tx.sender_address,
                )
                # BACKWARD-COMPATIBILITY.
                tx_nonce = (
                    0 if self.internal_tx.nonce is None else self.internal_tx.nonce
                )
                stark_assert_le(
                    nonce,
                    tx_nonce,
                    code=StarknetErrorCode.INVALID_TRANSACTION_NONCE,
                    message="Transaction's nonce must be greater than or equal to the last known nonce.",
                )

            def _check_balance(self, state: UpdatesTrackerState):
                balance = state.get_fee_token_balance(
                    storage_domain=StorageDomain.ON_CHAIN,
                    contract_address=self.internal_tx.sender_address,
                    fee_token_address=self.starknet_wrapper.fee_token.address,
                )
                stark_assert_le(
                    self.internal_tx.max_fee,
                    balance,
                    code=StarknetErrorCode.INSUFFICIENT_ACCOUNT_BALANCE,
                    message="Account balance must be greater or equal to the transaction's max_fee.",
                )

            def _validate(self, state: UpdatesTrackerState) -> ResourcesMapping:
                if isinstance(self.internal_tx, InternalDeployAccount):
                    # Run the entire transaction since a constructor call must precede the `validate`.
                    tx_execution_info = self.internal_tx.apply_concurrent_changes(
                        state=state,
                        general_config=self.starknet_wrapper.get_state().general_config,
                    )
                    for call_info in tx_execution_info.non_optional_calls:
                        stark_assert_call_succeeded(call_info=call_info)

                    actual_resources = tx_execution_info.actual_resources
                else:
                    resources_manager = ExecutionResourcesManager.empty()
                    validate_info, _ = self.internal_tx.run_validate_entrypoint(
                        state=state,
                        general_config=self.starknet_wrapper.get_state().general_config,
                        resources_manager=resources_manager,
                        remaining_gas=self.internal_tx.get_initial_gas(),
                    )
                    # Check can be removed when v0 transactions are disabled
                    if self.internal_tx.version > 0:
                        assert (
                            validate_info is not None
                        ), "validate_info must be not None for version > 0."
                        stark_assert_call_succeeded(call_info=validate_info)

                    actual_resources = calculate_tx_resources(
                        state=state,
                        resources_manager=resources_manager,
                        call_infos=[validate_info],
                        tx_type=self.internal_tx.tx_type,
                        fee_token_address=self.starknet_wrapper.fee_token.address,
                        is_nonce_increment=self.internal_tx.version > 0,
                        sender_address=self.internal_tx.sender_address,
                    )

                return actual_resources

            def _check_validation_fee(
                self, state: UpdatesTrackerState, actual_resources: ResourcesMapping
            ):
                # Check that max_fee is high enough to pay for the validation.
                actual_fee = calculate_tx_fee(
                    gas_price=state.block_info.gas_price,
                    general_config=self.starknet_wrapper.get_state().general_config,
                    resources=actual_resources,
                )

                stark_assert_le(
                    actual_fee,
                    self.internal_tx.max_fee,
                    code=StarknetErrorCode.INSUFFICIENT_MAX_FEE,
                    message="Max fee must be greater or equal to the validation's actual fee.",
                )

            def _validate_fee(self, external_tx: AccountTransaction):
                if (
                    external_tx.version != LEGACY_TX_VERSION
                    and external_tx.max_fee == 0
                    and not self.starknet_wrapper.config.allow_max_fee_zero
                ):
                    raise StarknetDevnetException(
                        code=StarknetErrorCode.OUT_OF_RANGE_FEE,
                        message="max_fee must be bigger than 0.",
                    )

            async def __aenter__(self):
                if self.internal_tx:
                    state = self.starknet_wrapper.get_state().state._copy()
                    loop = asyncio.get_running_loop()
                    state = UpdatesTrackerState(
                        state=StateSyncifier(async_state=state, loop=loop)
                    )
                    await asyncio.to_thread(
                        self._inner_perform_state_related_validations, state=state
                    )
                return self

            def _inner_perform_state_related_validations(
                self, state: UpdatesTrackerState
            ):
                self._check_nonce(state)
                self._check_balance(state)
                validation_resources = self._validate(state)
                if self.internal_tx.max_fee:
                    self._check_validation_fee(state, validation_resources)

            async def __aexit__(
                self,
                exc_type: Optional[Type[BaseException]],
                exc: Optional[BaseException],
                traceback: Optional[TracebackType],
            ):
                assert self.internal_tx is not None
                tx_hash = self.internal_tx.hash_value

                if exc_type:
                    if not isinstance(exc, StarkException):
                        raise StarknetDevnetException(
                            code=StarknetErrorCode.UNEXPECTED_FAILURE, message=str(exc)
                        ) from exc

                    # restore block info
                    self.starknet_wrapper.get_state().state.block_info = (
                        self.preserved_block_info
                    )

                    transaction = DevnetTransaction(
                        internal_tx=self.internal_tx,
                        status=TransactionStatus.REVERTED,
                        execution_status=ExecutionStatus.REVERTED,
                        finality_status=FinalityStatus.ACCEPTED_ON_L2,
                        execution_info=TransactionExecutionInfo.empty(),
                        transaction_hash=tx_hash,
                        block_number=0,  # Rejected txs have no block number
                        transaction_index=0,  # Rejected txs have no tx index
                        revert_error=exc.message,
                    )
                    self.starknet_wrapper.transactions.store(
                        transaction.transaction_hash, transaction
                    )
                else:
                    assert self.execution_info is not None
                    if self.execution_info.call_info:
                        await self.starknet_wrapper._register_new_contracts(
                            self.internal_calls,
                            tx_hash,
                            self.deployed_contracts,
                        )

                    state_update = await self.starknet_wrapper.update_pending_state(
                        deployed_contracts=self.deployed_contracts,
                        explicitly_declared=self.explicitly_declared,
                        explicitly_declared_old=self.explicitly_declared_old,
                        visited_storage_entries=self.visited_storage_entries,
                    )

                    next_block_number = (
                        self.starknet_wrapper.blocks.get_next_block_number()
                    )

                    transaction = DevnetTransaction(
                        internal_tx=self.internal_tx,
                        status=TransactionStatus.ACCEPTED_ON_L2,
                        execution_status=ExecutionStatus.SUCCEEDED,
                        finality_status=FinalityStatus.ACCEPTED_ON_L2,
                        execution_info=self.execution_info,
                        transaction_hash=tx_hash,
                        block_number=next_block_number,
                        transaction_index=len(self.starknet_wrapper.pending_txs),
                    )
                    self.starknet_wrapper.pending_txs.append(transaction)
                    self.starknet_wrapper.transactions.store(
                        transaction.transaction_hash, transaction
                    )

                    await self.starknet_wrapper.update_pending_block(state_update)

                    if not self.starknet_wrapper.config.blocks_on_demand:
                        await self.starknet_wrapper.generate_latest_block()

                logger.info(
                    "transaction execution info: %s",
                    pprint.pformat(self.execution_info.dump()),
                )
                logger.info(
                    "transaction receipt: %s",
                    pprint.pformat(transaction.get_receipt().dump()),
                )
                return True  # indicates the caught exception was handled successfully

        return TransactionHandler(self)

    async def deploy_account(self, external_tx: DeployAccount):
        """Deploys account and returns (address, tx_hash)"""

        account_address = calculate_contract_address_from_hash(
            salt=external_tx.contract_address_salt,
            class_hash=external_tx.class_hash,
            constructor_calldata=external_tx.constructor_calldata,
            deployer_address=0,
        )

        async with self.__get_transaction_handler(
            external_tx=external_tx
        ) as tx_handler:
            tx_handler.execution_info = await self.__deploy(tx_handler.internal_tx)
            tx_handler.internal_calls = (
                tx_handler.execution_info.call_info.internal_calls
            )

        return (
            account_address,
            tx_handler.internal_tx.hash_value,
        )

    async def invoke(self, external_tx: InvokeFunction):
        """Perform invoke according to specifications in `transaction`."""
        state = self.get_state()
        async with self.__get_transaction_handler(
            external_tx=external_tx
        ) as tx_handler:
            tx_handler.execution_info = await state.execute_tx(tx_handler.internal_tx)
            tx_handler.internal_calls = (
                tx_handler.execution_info.call_info.internal_calls
            )
            tx_handler.visited_storage_entries = (
                tx_handler.execution_info.get_visited_storage_entries()
            )

        return external_tx.sender_address, tx_handler.internal_tx.hash_value

    async def __get_query_state(self, block_id: BlockId = DEFAULT_BLOCK_ID):
        if block_id == PENDING_BLOCK_ID:
            return self.get_state()
        if block_id == LATEST_BLOCK_ID:
            return self.__latest_state

        assert isinstance(block_id, dict)
        if block_id.get("block_hash"):
            numeric_hash = self.blocks.get_numeric_hash(block_id.get("block_hash"))
            return self.blocks.get_state(numeric_hash)

        try:
            block_number = block_id.get("block_number")
            block = await self.blocks.get_by_number(int(block_number))
            return self.blocks.get_state(block.block_hash)
        except ValueError:
            pass

        raise StarknetDevnetException(
            code=StarknetErrorCode.INVALID_BLOCK_NUMBER,
            message=f"Invalid block id: {block_id}",
        )

    async def call(
        self,
        transaction: Union[CallFunction, InvokeFunction],
        block_id: BlockId = DEFAULT_BLOCK_ID,
    ):
        """Perform call according to specifications in `transaction`."""
        state = await self.__get_query_state(block_id)

        # property name different since starknet 0.11
        address = (
            transaction.contract_address
            if isinstance(transaction, CallFunction)
            else transaction.sender_address
        )

        call_info = await state.copy().execute_entry_point_raw(
            contract_address=address,
            selector=transaction.entry_point_selector,
            calldata=transaction.calldata,
            caller_address=0,
        )

        result = list(map(hex, call_info.retdata))
        return {"result": result}

    async def __deploy(self, deploy_tx: Union[InternalDeploy, InternalDeployAccount]):
        """
        Replacement for self.starknet.deploy that allows usage of InternalDeploy right away.
        This way InternalDeploy doesn't have to be created twice, calculating hash every time.
        """
        state = self.get_state()
        tx_execution_info = await state.execute_tx(tx=deploy_tx)
        return tx_execution_info

    async def _register_new_contracts(
        self,
        internal_calls: List[Union[FunctionInvocation, CallInfo]],
        tx_hash: int,
        deployed_contracts: List[ContractAddressHashPair],
    ):
        for internal_call in internal_calls:
            if internal_call.entry_point_type == EntryPointType.CONSTRUCTOR:
                deployed_contracts.append(
                    ContractAddressHashPair(
                        address=internal_call.contract_address,
                        class_hash=internal_call.class_hash,
                    )
                )
            await self._register_new_contracts(
                internal_call.internal_calls, tx_hash, deployed_contracts
            )

    async def get_class_by_hash(self, class_hash: int) -> dict:
        """Return contract class given class hash"""
        if class_hash in self._contract_classes:
            # check if locally present
            return self._contract_classes[class_hash].dump()

        return await self.origin.get_class_by_hash(class_hash)

    async def get_compiled_class_by_class_hash(self, class_hash: int) -> CompiledClass:
        """
        Return compiled class given the class hash (sierra hash).
        Should report an undeclared class if given the hash of a deprecated class
        """
        state = self.get_state().state

        # first handle the case of artifact being locally present
        if class_hash in self._contract_classes:
            contract_class = self._contract_classes[class_hash]
            if isinstance(contract_class, DeprecatedCompiledClass):
                # should raise if class hash does not belong to sierra of a cairo 1 contract
                raise UndeclaredClassDevnetException(class_hash)

            compiled_class_hash = await state.get_compiled_class_hash(class_hash)
            return await state.get_compiled_class(compiled_class_hash)

        try:
            # directly on state_reader to ensure overridden method is called if forking
            compiled_class = await state.state_reader.get_compiled_class_by_class_hash(
                class_hash
            )
            if isinstance(compiled_class, CompiledClass):
                return compiled_class
        except AssertionError:
            # the received hash is compiled_class_hash of a cairo1 class
            pass
        raise UndeclaredClassDevnetException(class_hash)

    async def get_class_hash_at(
        self, contract_address: int, block_id: BlockId = DEFAULT_BLOCK_ID
    ) -> int:
        """Return class hash given the contract address"""
        state = await self.__get_query_state(block_id)
        cached_state = state.state
        class_hash = await cached_state.get_class_hash_at(contract_address)

        if not class_hash:
            raise StarknetDevnetException(
                code=StarknetErrorCode.UNINITIALIZED_CONTRACT,
                message=f"Contract with address {contract_address} is not deployed.",
            )
        return class_hash

    async def get_class_by_address(
        self, contract_address: int, block_id: BlockId = DEFAULT_BLOCK_ID
    ) -> dict:
        """Return contract class given the contract address"""
        class_hash = await self.get_class_hash_at(contract_address, block_id)
        return await self.get_class_by_hash(class_hash)

    async def get_code(
        self, contract_address: int, block_id: BlockId = DEFAULT_BLOCK_ID
    ) -> dict:
        """Return code dict given the contract address"""
        try:
            contract_class = await self.get_class_by_address(contract_address, block_id)
            bytecode = (
                contract_class["program"]["data"]  # cairo 0
                if "program" in contract_class
                else contract_class["sierra_program"]  # cairo 1
            )
            result_dict = {
                "abi": contract_class["abi"],
                "bytecode": bytecode,
            }
        except StarkException as err:
            if err.code != StarknetErrorCode.UNINITIALIZED_CONTRACT:
                raise
            result_dict = {"abi": {}, "bytecode": []}

        return result_dict

    async def get_storage_at(
        self,
        contract_address: int,
        key: int,
        block_id: BlockId = DEFAULT_BLOCK_ID,
    ) -> Felt:
        """
        Returns the storage identified by `key` from the contract at `contract_address`.
        """
        state = await self.__get_query_state(block_id)
        return hex(
            await state.state.get_storage_at(
                StorageDomain.ON_CHAIN, contract_address, key
            )
        )

    async def load_messaging_contract_in_l1(
        self, network_url: str, contract_address: str, network_id: str
    ) -> dict:
        """Loads the messaging contract at `contract_address`"""
        return self.l1l2.load_l1_messaging_contract(
            self.starknet, network_url, contract_address, network_id
        )

    async def consume_message_from_l2(
        self, from_address: int, to_address: int, payload: List[int]
    ) -> str:
        """Mocks the L1 contract function consumeMessageFromL2."""
        state = self.get_state()

        starknet_message = StarknetMessageToL1(
            from_address=from_address,
            to_address=to_address,
            payload=payload,
        )
        message_hash = starknet_message.get_hash()
        state.consume_message_hash(message_hash=message_hash)
        return message_hash

    async def mock_message_to_l2(self, transaction: InternalL1Handler) -> dict:
        """Handles L1 to L2 message mock endpoint"""

        state = self.get_state()
        # Execute transactions inside StarknetWrapper
        async with self.__get_transaction_handler() as tx_handler:
            tx_handler.internal_tx = transaction
            tx_handler.execution_info = await state.execute_tx(tx_handler.internal_tx)
            tx_handler.internal_calls = (
                tx_handler.execution_info.call_info.internal_calls
            )

        return transaction.hash_value

    async def postman_flush(self) -> dict:
        """Handles all pending L1 <> L2 messages and sends them to the other layer."""

        state = self.get_state()
        # Generate transactions in PostmanWrapper
        parsed_l1_l2_messages, transactions_to_execute = await self.l1l2.flush(state)
        tx_hashes = []

        # Execute transactions inside StarknetWrapper
        if parsed_l1_l2_messages and transactions_to_execute:
            for transaction in transactions_to_execute:
                tx_hashes.append(hex(transaction.hash_value))
                async with self.__get_transaction_handler() as tx_handler:
                    tx_handler.internal_tx = transaction
                    tx_handler.execution_info = await state.execute_tx(
                        tx_handler.internal_tx
                    )
                    tx_handler.internal_calls = (
                        tx_handler.execution_info.call_info.internal_calls
                    )

        parsed_l1_l2_messages["generated_l2_transactions"] = tx_hashes
        return parsed_l1_l2_messages

    async def update_pending_block(self, state_update: BlockStateUpdate = None):
        """Update pending block"""
        await self.blocks.generate_pending(
            transactions=self.pending_txs,
            state=self.get_state(),
            state_update=state_update,
        )

    async def generate_latest_block(self, block_hash=None) -> StarknetBlock:
        """
        Generate new block with pending transactions or empty block.
        Block hash can be specified in special cases.
        """

        # Store transactions and clear pending txs
        state = self.get_state()
        if self.blocks.is_block_pending():
            block = await self.blocks.store_pending(state, block_hash=block_hash)
        else:
            # if no pending, default to creating an empty block
            assert not self.pending_txs
            block = await self.create_empty_block()

        for transaction in self.pending_txs:
            transaction.status = TransactionStatus.ACCEPTED_ON_L2
            transaction.set_block(block=block)

        # Update latest state before block generation
        self.__latest_state = state.copy()

        self.pending_txs = []

        return block

    async def calculate_trace_and_fee(
        self,
        external_tx: InvokeFunction,
        skip_validate: bool,
        block_id: BlockId = DEFAULT_BLOCK_ID,
    ):
        """Calculates trace and fee by simulating tx on state copy."""
        traces, fees = await self.calculate_traces_and_fees(
            [external_tx], skip_validate=skip_validate, block_id=block_id
        )
        assert len(traces) == len(fees) == 1
        return traces[0], fees[0]

    async def calculate_traces_and_fees(
        self,
        external_txs: List[InvokeFunction],
        skip_validate: bool,
        block_id: BlockId = DEFAULT_BLOCK_ID,
    ):
        """Calculates traces and fees by simulating tx on state copy.
        Uses the resulting state for each consecutive estimation"""
        state = await self.__get_query_state(block_id)
        cached_state_copy = state.state

        traces = []
        fee_estimation_infos = []

        for external_tx in external_txs:
            # pylint: disable=protected-access
            cached_state_copy = cached_state_copy._copy()
            try:
                internal_tx = InternalInvokeFunctionForSimulate.create_for_simulate(
                    external_tx,
                    state.general_config,
                    skip_validate=skip_validate,
                )
            except AssertionError as error:
                raise StarknetDevnetException(
                    code=StarkErrorCode.MALFORMED_REQUEST,
                    status_code=400,
                    message="Invalid format of fee estimation request",
                ) from error

            execution_info = await internal_tx.apply_state_updates(
                cached_state_copy,
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
                revert_error=execution_info.revert_error,
            )
            traces.append(trace)

            fee_estimation_info = get_fee_estimation_info(
                execution_info.actual_fee, state.state.block_info.gas_price
            )
            fee_estimation_infos.append(fee_estimation_info)

        assert len(traces) == len(fee_estimation_infos) == len(external_txs)
        return traces, fee_estimation_infos

    async def estimate_message_fee(
        self, call: CallL1Handler, block_id: BlockId = DEFAULT_BLOCK_ID
    ):
        """Estimate fee of message from L1 to L2"""
        state = await self.__get_query_state(block_id)
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

    def __set_gas_price(self, gas_price: int):
        """Sets gas price to `gas_price`."""
        self.block_info_generator.set_gas_price(gas_price)

    async def get_nonce(
        self, contract_address: int, block_id: BlockId = DEFAULT_BLOCK_ID
    ):
        """Returns nonce of contract with `contract_address`"""
        state = await self.__get_query_state(block_id)
        return await state.state.get_nonce_at(StorageDomain.ON_CHAIN, contract_address)

    async def __predeclare_starknet_cli_account(self):
        """Predeclares the account class used by Starknet CLI"""
        state = self.get_state().state
        state.compiled_classes[STARKNET_CLI_ACCOUNT_CLASS_HASH] = oz_account_class
        if self.config.verbose or not self.config.hide_predeployed_contracts:
            print("Predeclared Starknet CLI account: ", flush=True)
            print(f"Class hash: {hex(STARKNET_CLI_ACCOUNT_CLASS_HASH)}\n", flush=True)

    async def __deploy_chargeable_account(self):
        if await self.is_deployed(ChargeableAccount.ADDRESS):
            warn("Chargeable account already deployed")
        else:
            await ChargeableAccount(self).deploy()

    async def is_deployed(self, address: int) -> bool:
        """Check if the contract is deployed."""
        assert isinstance(address, int)
        cached_state = self.get_state().state
        class_hash = await cached_state.get_class_hash_at(address)
        return bool(class_hash)

    async def abort_blocks(self, starting_block: StarknetBlock) -> str:
        """
        Abort blocks.
        """
        # Check if genesis block can be aborted.
        if starting_block.block_number == self.genesis_block_number:
            raise StarknetDevnetException(
                code=StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID,
                message="Aborting genesis block is not supported.",
            )

        # Check if blocks can be aborted in fork mode.
        if (
            self.config.fork_block
            and self.config.fork_block >= starting_block.block_number
        ):
            raise StarknetDevnetException(
                code=StarknetErrorCode.OUT_OF_RANGE_BLOCK_ID,
                message="Aborting forked blocks is not supported.",
            )

        # Create new block with pending transactions if possible.
        # We need to store them so later we can change the status to REJECTED.
        if self.blocks.is_block_pending():
            await self.generate_latest_block()

        aborted_blocks = []
        last_block = await self.blocks.get_last_block()

        # Before the while loop to abort blocks, check if block numbers are set.
        if not (last_block.block_number and starting_block.block_number):
            raise StarknetDevnetException(
                code=StarknetErrorCode.BLOCK_NOT_FOUND,
                status_code=400,
                message="Block cannot be aborted. Make sure you are aborting an accepted block.",
            )

        # Abort blocks from latest to starting (iterating backwards).
        reached_starting_block = False
        while not reached_starting_block:
            reached_starting_block = (
                last_block.block_number == starting_block.block_number
            )

            # Abort latest_block.
            aborted_block_hash = await self.blocks.abort_latest_block(
                hex(last_block.block_hash)
            )

            # Revert transactions.
            for transaction in last_block.transactions:
                await self.transactions.revert_transaction_in_aborted_block(
                    tx_hash=transaction.transaction_hash
                )

            aborted_blocks.append(hex(aborted_block_hash))
            parent = await self.blocks.get_by_hash(hex(last_block.parent_block_hash))

            if parent.block_number is not None:
                last_block = parent
            else:
                break

        # Revert state.
        self.starknet.state = self.blocks.get_state(last_block.block_hash)

        return aborted_blocks
