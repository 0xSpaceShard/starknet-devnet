use std::collections::HashMap;

use blockifier::block_context::BlockContext;
use blockifier::execution::entry_point::CallEntryPoint;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::errors::TransactionPreValidationError;
use blockifier::transaction::objects::TransactionExecutionInfo;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
use starknet_api::transaction::Fee;
use starknet_rs_core::types::{
    BlockId, MsgFromL1, TransactionExecutionStatus, TransactionFinalityStatus,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_ff::FieldElement;
use starknet_rs_signers::Signer;
use starknet_types::chain_id::ChainId;
use starknet_types::constants::{
    BITWISE_BUILTIN_NAME, EC_OP_BUILTIN_NAME, HASH_BUILTIN_NAME, KECCAK_BUILTIN_NAME, N_STEPS,
    OUTPUT_BUILTIN_NAME, POSEIDON_BUILTIN_NAME, RANGE_CHECK_BUILTIN_NAME,
    SEGMENT_ARENA_BUILTIN_NAME, SIGNATURE_BUILTIN_NAME,
};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::emitted_event::EmittedEvent;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{Block, BlockHeader};
use starknet_types::rpc::estimate_message_fee::FeeEstimateWrapper;
use starknet_types::rpc::state::ThinStateDiff;
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use starknet_types::rpc::transactions::{
    BroadcastedTransaction, BroadcastedTransactionCommon, DeclareTransaction,
    DeclareTransactionTrace, DeployAccountTransactionTrace, ExecutionInvocation,
    FunctionInvocation, InvokeTransactionTrace, SimulatedTransaction, SimulationFlag, Transaction,
    TransactionTrace, Transactions,
};
use starknet_types::traits::HashProducer;
use tracing::{error, warn};

use self::predeployed::initialize_erc20;
use self::starknet_config::{DumpOn, StarknetConfig};
use crate::account::Account;
use crate::blocks::{StarknetBlock, StarknetBlocks};
use crate::constants::{
    CHARGEABLE_ACCOUNT_ADDRESS, CHARGEABLE_ACCOUNT_PRIVATE_KEY, DEVNET_DEFAULT_CHAIN_ID,
    ERC20_CONTRACT_ADDRESS,
};
use crate::error::{DevnetResult, Error, TransactionValidationError};
use crate::predeployed_accounts::PredeployedAccounts;
use crate::raw_execution::{Call, RawExecution};
use crate::state::state_diff::StateDiff;
use crate::state::state_update::StateUpdate;
use crate::state::StarknetState;
use crate::traits::{
    AccountGenerator, Accounted, Deployed, HashIdentified, HashIdentifiedMut, StateChanger,
    StateExtractor,
};
use crate::transactions::{StarknetTransaction, StarknetTransactions};

mod add_declare_transaction;
mod add_deploy_account_transaction;
mod add_invoke_transaction;
mod dump;
mod estimations;
mod events;
mod get_class_impls;
mod predeployed;
pub mod starknet_config;
mod state_update;

pub struct Starknet {
    pub(in crate::starknet) state: StarknetState,
    predeployed_accounts: PredeployedAccounts,
    pub(in crate::starknet) block_context: BlockContext,
    blocks: StarknetBlocks,
    pub transactions: StarknetTransactions,
    pub config: StarknetConfig,
    pub pending_block_timestamp_shift: i64,
}

impl Default for Starknet {
    fn default() -> Self {
        Self {
            block_context: Self::init_block_context(
                0,
                ERC20_CONTRACT_ADDRESS,
                DEVNET_DEFAULT_CHAIN_ID,
            ),
            state: Default::default(),
            predeployed_accounts: Default::default(),
            blocks: Default::default(),
            transactions: Default::default(),
            config: Default::default(),
            pending_block_timestamp_shift: 0,
        }
    }
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_fee_contract = predeployed::create_erc20()?;
        let udc_contract = predeployed::create_udc()?;

        erc20_fee_contract.deploy(&mut state)?;
        initialize_erc20(&mut state)?;

        udc_contract.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance,
            erc20_fee_contract.get_address(),
        );

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            config.account_contract_class_hash,
            config.account_contract_class.clone(),
        )?;
        for account in accounts {
            account.deploy(&mut state)?;
            account.set_initial_balance(&mut state)?;
        }

        let chargeable_account = Account::new_chargeable(erc20_fee_contract.get_address())?;
        chargeable_account.deploy(&mut state)?;
        chargeable_account.set_initial_balance(&mut state)?;

        // copy already modified state to cached state
        state.clear_dirty_state();

        let mut this = Self {
            state,
            predeployed_accounts,
            block_context: Self::init_block_context(
                config.gas_price,
                ERC20_CONTRACT_ADDRESS,
                config.chain_id,
            ),
            blocks: StarknetBlocks::default(),
            transactions: StarknetTransactions::default(),
            config: config.clone(),
            pending_block_timestamp_shift: 0,
        };

        this.restart_pending_block()?;

        // Load starknet transactions
        if this.config.dump_path.is_some() && this.config.re_execute_on_init {
            // Try to load transactions from dump_path, if there is no file skip this step
            match this.load_transactions() {
                Ok(txs) => this.re_execute(txs)?,
                Err(Error::FileNotFound) => {}
                Err(err) => return Err(err),
            };
        }

        Ok(this)
    }

    pub fn restart(&mut self) -> DevnetResult<()> {
        self.config.re_execute_on_init = false;
        *self = Starknet::new(&self.config)?;
        Ok(())
    }

    pub fn get_predeployed_accounts(&self) -> Vec<Account> {
        self.predeployed_accounts.get_accounts().to_vec()
    }

    // Update block context
    // Initialize values for new pending block
    pub(crate) fn generate_pending_block(&mut self) -> DevnetResult<()> {
        Self::update_block_context(&mut self.block_context);
        self.restart_pending_block()?;

        Ok(())
    }

    /// Transfer data from pending block into new block and save it to blocks collection
    /// Returns the new block number
    pub(crate) fn generate_new_block(
        &mut self,
        state_diff: StateDiff,
        timestamp: Option<u64>,
    ) -> DevnetResult<BlockNumber> {
        let mut new_block = self.pending_block().clone();

        // set new block header
        new_block.set_block_hash(new_block.generate_hash()?);
        new_block.status = BlockStatus::AcceptedOnL2;

        // set block timestamp and context block timestamp for contract execution
        let block_timestamp = match timestamp {
            Some(timestamp) => BlockTimestamp(timestamp),
            None => BlockTimestamp(
                (Starknet::get_unix_timestamp_as_seconds() as i64
                    + self.pending_block_timestamp_shift) as u64,
            ),
        };
        new_block.set_timestamp(block_timestamp);
        self.block_context.block_timestamp = block_timestamp;

        let new_block_number = new_block.block_number();

        // update txs block hash block number for each transaction in the pending block
        new_block.get_transactions().iter().for_each(|tx_hash| {
            if let Some(tx) = self.transactions.get_by_hash_mut(tx_hash) {
                tx.block_hash = Some(new_block.header.block_hash.0.into());
                tx.block_number = Some(new_block_number);
                tx.finality_status = TransactionFinalityStatus::AcceptedOnL2;
            } else {
                error!("Transaction is not present in the transactions collection");
            }
        });

        // insert pending block in the blocks collection and connect it to the state diff
        self.blocks.insert(new_block, state_diff);
        // save into blocks state archive

        let deep_cloned_state = self.state.clone();
        self.blocks.save_state_at(new_block_number, deep_cloned_state);

        Ok(new_block_number)
    }

    /// Handles transaction result either Ok or Error and updates the state accordingly.
    ///
    /// # Arguments
    ///
    /// * `transaction` - Transaction to be added in the collection of transactions.
    /// * `transaction_result` - Result with transaction_execution_info
    pub(crate) fn handle_transaction_result(
        &mut self,
        transaction: Transaction,
        transaction_result: Result<
            TransactionExecutionInfo,
            blockifier::transaction::errors::TransactionExecutionError,
        >,
    ) -> DevnetResult<()> {
        let transaction_hash = *transaction.get_transaction_hash();

        match transaction_result {
            Ok(tx_info) => {
                // If transaction is not reverted
                // then save the contract class in the state cache for Declare V1/V2 transactions
                if !tx_info.is_reverted() {
                    match &transaction {
                        Transaction::Declare(DeclareTransaction::Version1(declare_v1)) => {
                            self.state.contract_classes.insert(
                                declare_v1.class_hash,
                                declare_v1.contract_class.clone().into(),
                            );
                        }
                        Transaction::Declare(DeclareTransaction::Version2(declare_v2)) => {
                            self.state.contract_classes.insert(
                                declare_v2.class_hash,
                                declare_v2.contract_class.clone().into(),
                            );
                        }
                        _ => {}
                    };
                }
                self.handle_accepted_transaction(&transaction_hash, &transaction, tx_info)
            }
            Err(tx_err) => {
                /// utility to avoid duplication
                fn match_tx_fee_error(
                    err: blockifier::transaction::errors::TransactionFeeError,
                ) -> DevnetResult<()> {
                    match err {
                        blockifier::transaction::errors::TransactionFeeError::FeeTransferError { .. }
                        | blockifier::transaction::errors::TransactionFeeError::MaxFeeTooLow { .. } => Err(
                            TransactionValidationError::InsufficientMaxFee.into()
                        ),
                        blockifier::transaction::errors::TransactionFeeError::MaxFeeExceedsBalance { .. } => Err(
                            TransactionValidationError::InsufficientAccountBalance.into()
                        ),
                        _ => Err(err.into())
                    }
                }

                // based on this https://community.starknet.io/t/efficient-utilization-of-sequencer-capacity-in-starknet-v0-12-1/95607#the-validation-phase-in-the-gateway-5
                // we should not save transactions that failed with one of the following errors
                match tx_err {
                    blockifier::transaction::errors::TransactionExecutionError::TransactionPreValidationError(
                        TransactionPreValidationError::InvalidNonce { .. }
                    ) => Err(TransactionValidationError::InvalidTransactionNonce.into()),
                    blockifier::transaction::errors::TransactionExecutionError::FeeCheckError { .. } =>
                        Err(TransactionValidationError::InsufficientMaxFee.into()),
                    blockifier::transaction::errors::TransactionExecutionError::TransactionPreValidationError(
                        TransactionPreValidationError::TransactionFeeError(err)
                    ) => match_tx_fee_error(err),
                    blockifier::transaction::errors::TransactionExecutionError::TransactionFeeError(err)
                      => match_tx_fee_error(err),
                    blockifier::transaction::errors::TransactionExecutionError::ValidateTransactionError(..) =>
                        Err(TransactionValidationError::ValidationFailure.into()),
                    _ => Err(tx_err.into())
                }
            }
        }
    }

    /// Handles suceeded and reverted transactions.
    /// The tx is stored and potentially dumped.
    /// A new block is generated.
    pub(crate) fn handle_accepted_transaction(
        &mut self,
        transaction_hash: &TransactionHash,
        transaction: &Transaction,
        tx_info: TransactionExecutionInfo,
    ) -> DevnetResult<()> {
        let transaction_to_add = StarknetTransaction::create_accepted(transaction, tx_info);

        // add accepted transaction to pending block
        self.blocks.pending_block.add_transaction(*transaction_hash);

        self.transactions.insert(transaction_hash, transaction_to_add);

        let state_difference = self.state.extract_state_diff_from_pending_state()?;
        // apply state changes from cached state
        self.state.apply_state_difference(state_difference.clone())?;
        // make cached state part of "persistent" state
        self.state.clear_dirty_state();
        // create new block from pending one
        self.generate_new_block(state_difference, None)?;
        // clear pending block information
        self.generate_pending_block()?;

        if self.config.dump_on == Some(DumpOn::Transaction) {
            self.dump_transaction(transaction)?;
        }

        Ok(())
    }

    fn init_block_context(
        gas_price: u64,
        fee_token_address: &str,
        chain_id: ChainId,
    ) -> BlockContext {
        use starknet_api::core::{ContractAddress, PatriciaKey};
        use starknet_api::hash::StarkHash;
        use starknet_api::{contract_address, patricia_key};

        // Create a BlockContext based on BlockContext::create_for_testing()
        const N_STEPS_FEE_WEIGHT: f64 = 0.01;
        BlockContext {
            chain_id: chain_id.into(),
            block_number: BlockNumber(0),
            block_timestamp: BlockTimestamp(0),
            sequencer_address: contract_address!("0x1000"),
            fee_token_addresses: blockifier::block_context::FeeTokenAddresses {
                eth_fee_token_address: contract_address!(fee_token_address),
                strk_fee_token_address: contract_address!("0x1002"),
            },
            vm_resource_fee_cost: std::sync::Arc::new(HashMap::from([
                (N_STEPS.to_string(), N_STEPS_FEE_WEIGHT),
                (OUTPUT_BUILTIN_NAME.to_string(), 0.0),
                (HASH_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 32.0),
                (RANGE_CHECK_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 16.0),
                (SIGNATURE_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 2048.0),
                (BITWISE_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 64.0),
                (EC_OP_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 1024.0),
                (POSEIDON_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 32.0),
                (SEGMENT_ARENA_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 10.0),
                (KECCAK_BUILTIN_NAME.to_string(), N_STEPS_FEE_WEIGHT * 2048.0), // 2**11
            ])),
            gas_prices: blockifier::block_context::GasPrices {
                eth_l1_gas_price: gas_price as u128,
                strk_l1_gas_price: gas_price as u128,
            },
            invoke_tx_max_n_steps: 4_000_000_u32,
            validate_max_n_steps: 1_000_000_u32,
            max_recursion_depth: 50,
        }
    }

    /// Update block context block_number with the next one
    /// # Arguments
    /// * `block_context` - BlockContext to be updated
    fn update_block_context(block_context: &mut BlockContext) {
        block_context.block_number = block_context.block_number.next();
    }

    fn pending_block(&self) -> &StarknetBlock {
        &self.blocks.pending_block
    }

    /// Restarts pending block with information from block_context
    fn restart_pending_block(&mut self) -> DevnetResult<()> {
        let mut block = StarknetBlock::create_pending_block();

        block.header.block_number = self.block_context.block_number;
        block.header.gas_price = GasPrice(self.block_context.gas_prices.eth_l1_gas_price);
        block.header.sequencer = self.block_context.sequencer_address;

        self.blocks.pending_block = block;

        Ok(())
    }

    fn get_state_at(&self, block_id: &BlockId) -> DevnetResult<&StarknetState> {
        match block_id {
            BlockId::Tag(_) => Ok(&self.state),
            _ => {
                let block = self.blocks.get_by_block_id(*block_id).ok_or(Error::NoBlock)?;
                let state = self
                    .blocks
                    .num_to_state
                    .get(&block.block_number())
                    .ok_or(Error::NoStateAtBlock { block_number: block.block_number().0 })?;
                Ok(state)
            }
        }
    }

    pub fn get_class_hash_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<ClassHash> {
        get_class_impls::get_class_hash_at_impl(self, block_id, contract_address)
    }

    pub fn get_class(
        &self,
        block_id: BlockId,
        class_hash: ClassHash,
    ) -> DevnetResult<ContractClass> {
        get_class_impls::get_class_impl(self, block_id, class_hash)
    }

    pub fn get_class_at(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<ContractClass> {
        get_class_impls::get_class_at_impl(self, block_id, contract_address)
    }

    pub fn call(
        &self,
        block_id: BlockId,
        contract_address: Felt,
        entrypoint_selector: Felt,
        calldata: Vec<Felt>,
    ) -> DevnetResult<Vec<Felt>> {
        let state = self.get_state_at(&block_id)?;

        if !self.state.is_contract_deployed(&ContractAddress::new(contract_address)?) {
            return Err(Error::ContractNotFound);
        }

        let call = CallEntryPoint {
            calldata: starknet_api::transaction::Calldata(std::sync::Arc::new(
                calldata.iter().map(|f| f.into()).collect(),
            )),
            storage_address: starknet_api::hash::StarkFelt::from(contract_address).try_into()?,
            entry_point_selector: starknet_api::core::EntryPointSelector(
                entrypoint_selector.into(),
            ),
            initial_gas: blockifier::transaction::transaction_execution::Transaction::initial_gas(),
            ..Default::default()
        };

        let mut execution_resources =
            blockifier::execution::entry_point::ExecutionResources::default();
        let mut execution_context =
            blockifier::execution::entry_point::EntryPointExecutionContext::new(
                &self.block_context,
                &blockifier::transaction::objects::AccountTransactionContext::Deprecated(
                    blockifier::transaction::objects::DeprecatedAccountTransactionContext::default(
                    ),
                ),
                blockifier::execution::common_hints::ExecutionMode::Execute,
                true,
            )?;
        let res = call
            .execute(&mut state.clone().state, &mut execution_resources, &mut execution_context)
            .map_err(|err| {
                Error::BlockifierTransactionError(blockifier::transaction::errors::TransactionExecutionError::EntryPointExecutionError(err))
            })?;

        Ok(res.execution.retdata.0.into_iter().map(Felt::from).collect())
    }

    pub fn estimate_fee(
        &self,
        block_id: BlockId,
        transactions: &[BroadcastedTransaction],
    ) -> DevnetResult<Vec<FeeEstimateWrapper>> {
        estimations::estimate_fee(self, block_id, transactions, None, None)
    }

    pub fn estimate_message_fee(
        &self,
        block_id: BlockId,
        message: MsgFromL1,
    ) -> DevnetResult<FeeEstimateWrapper> {
        estimations::estimate_message_fee(self, block_id, message)
    }

    pub fn add_declare_transaction_v1(
        &mut self,
        declare_transaction: BroadcastedDeclareTransactionV1,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        add_declare_transaction::add_declare_transaction_v1(self, declare_transaction)
    }

    pub fn add_declare_transaction_v2(
        &mut self,
        declare_transaction: BroadcastedDeclareTransactionV2,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        add_declare_transaction::add_declare_transaction_v2(self, declare_transaction)
    }

    /// returning the chain id as object
    pub fn chain_id(&self) -> ChainId {
        self.config.chain_id
    }

    pub fn add_deploy_account_transaction(
        &mut self,
        deploy_account_transaction: BroadcastedDeployAccountTransaction,
    ) -> DevnetResult<(TransactionHash, ContractAddress)> {
        add_deploy_account_transaction::add_deploy_account_transaction(
            self,
            deploy_account_transaction,
        )
    }

    pub fn add_invoke_transaction(
        &mut self,
        invoke_transaction: BroadcastedInvokeTransaction,
    ) -> DevnetResult<TransactionHash> {
        add_invoke_transaction::add_invoke_transaction(self, invoke_transaction)
    }

    /// Creates an invoke tx for minting, using the chargeable account.
    pub async fn mint(&mut self, address: ContractAddress, amount: u128) -> DevnetResult<Felt> {
        let sufficiently_big_max_fee: u128 = self.config.gas_price as u128 * 1_000_000;
        let chargeable_address_felt = Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS)?;
        let nonce =
            self.state.state.get_nonce_at(starknet_api::core::ContractAddress::try_from(
                starknet_api::hash::StarkFelt::from(chargeable_address_felt),
            )?)?;

        let calldata = vec![
            Felt::from(address).into(),
            FieldElement::from(amount), // `low` part of Uint256
            FieldElement::from(0u32),   // `high` part
        ];

        let erc20_address_felt = Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS)?;
        let raw_execution = RawExecution {
            calls: vec![Call {
                to: erc20_address_felt.into(),
                selector: get_selector_from_name("mint").unwrap(),
                calldata: calldata.clone(),
            }],
            nonce: Felt::from(nonce.0).into(),
            max_fee: FieldElement::from(sufficiently_big_max_fee),
        };

        // generate msg hash (not the same as tx hash)
        let chain_id_felt: Felt = self.config.chain_id.to_felt();
        let msg_hash_felt =
            raw_execution.transaction_hash(chain_id_felt.into(), chargeable_address_felt.into());

        // generate signature by signing the msg hash
        let signer = starknet_rs_signers::LocalWallet::from(
            starknet_rs_signers::SigningKey::from_secret_scalar(
                FieldElement::from_hex_be(CHARGEABLE_ACCOUNT_PRIVATE_KEY).unwrap(),
            ),
        );
        let signature = signer.sign_hash(&msg_hash_felt).await?;

        let invoke_tx = BroadcastedInvokeTransaction {
            sender_address: ContractAddress::new(chargeable_address_felt)?,
            calldata: raw_execution.raw_calldata().into_iter().map(|c| c.into()).collect(),
            common: BroadcastedTransactionCommon {
                max_fee: Fee(sufficiently_big_max_fee),
                version: Felt::from(1),
                signature: vec![signature.r.into(), signature.s.into()],
                nonce: nonce.0.into(),
            },
        };

        // apply the invoke tx
        self.add_invoke_transaction(invoke_tx)
    }

    pub fn block_state_update(&self, block_id: BlockId) -> DevnetResult<StateUpdate> {
        state_update::state_update_by_block_id(self, block_id)
    }

    pub fn get_block_txs_count(&self, block_id: BlockId) -> DevnetResult<u64> {
        let block = self.blocks.get_by_block_id(block_id).ok_or(Error::NoBlock)?;

        Ok(block.get_transactions().len() as u64)
    }

    pub fn contract_nonce_at_block(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<Felt> {
        let state = self.get_state_at(&block_id)?;
        state.get_nonce(&contract_address)
    }

    pub fn contract_storage_at_block(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
        storage_key: PatriciaKey,
    ) -> DevnetResult<Felt> {
        let state = self.get_state_at(&block_id)?;
        state.get_storage(ContractStorageKey::new(contract_address, storage_key))
    }

    pub fn get_block(&self, block_id: BlockId) -> DevnetResult<StarknetBlock> {
        let block = self.blocks.get_by_block_id(block_id).ok_or(Error::NoBlock)?;
        Ok(block.clone())
    }

    pub fn get_block_with_transactions(&self, block_id: BlockId) -> DevnetResult<Block> {
        let block = self.blocks.get_by_block_id(block_id).ok_or(Error::NoBlock)?;
        let transactions = block
            .get_transactions()
            .iter()
            .map(|transaction_hash| {
                self.transactions
                    .get_by_hash(*transaction_hash)
                    .ok_or(Error::NoTransaction)
                    .map(|transaction| transaction.inner.clone())
            })
            .collect::<DevnetResult<Vec<Transaction>>>()?;

        Ok(Block {
            status: *block.status(),
            header: BlockHeader::from(block),
            transactions: Transactions::Full(transactions),
        })
    }

    pub fn get_transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        index: u64,
    ) -> DevnetResult<&Transaction> {
        let block = self.get_block(block_id)?;
        let transaction_hash = block
            .get_transactions()
            .get(index as usize)
            .ok_or(Error::InvalidTransactionIndexInBlock)?;

        self.get_transaction_by_hash(*transaction_hash)
    }

    pub fn get_latest_block(&self) -> DevnetResult<StarknetBlock> {
        let block = self
            .blocks
            .get_by_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
            .ok_or(crate::error::Error::NoBlock)?;

        Ok(block.clone())
    }

    pub fn get_transaction_by_hash(&self, transaction_hash: Felt) -> DevnetResult<&Transaction> {
        self.transactions
            .get_by_hash(transaction_hash)
            .map(|starknet_transaction| &starknet_transaction.inner)
            .ok_or(Error::NoTransaction)
    }

    pub fn get_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<ContractAddress>,
        keys: Option<Vec<Vec<Felt>>>,
        skip: usize,
        limit: Option<usize>,
    ) -> DevnetResult<(Vec<EmittedEvent>, bool)> {
        events::get_events(self, from_block, to_block, address, keys, skip, limit)
    }

    pub fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> DevnetResult<TransactionReceipt> {
        let transaction_to_map =
            self.transactions.get(&transaction_hash).ok_or(Error::NoTransaction)?;

        transaction_to_map.get_receipt()
    }

    pub fn get_transaction_execution_and_finality_status(
        &self,
        transaction_hash: TransactionHash,
    ) -> DevnetResult<(TransactionExecutionStatus, TransactionFinalityStatus)> {
        let transaction = self.transactions.get(&transaction_hash).ok_or(Error::NoTransaction)?;

        Ok((transaction.execution_result.status(), transaction.finality_status))
    }

    pub fn simulate_transactions(
        &self,
        block_id: BlockId,
        transactions: &[BroadcastedTransaction],
        simulation_flags: Vec<SimulationFlag>,
    ) -> DevnetResult<Vec<SimulatedTransaction>> {
        let mut state = self.get_state_at(&block_id)?.clone();
        let chain_id = self.chain_id().to_felt();

        let mut skip_validate = false;
        let mut skip_fee_charge = false;
        for flag in simulation_flags.iter() {
            match flag {
                SimulationFlag::SkipValidate => {
                    skip_validate = true;
                    warn!("SKIP_VALIDATE chosen in simulation, but does not affect fee estimation");
                }
                SimulationFlag::SkipFeeCharge => skip_fee_charge = true,
            }
        }

        let mut transactions_traces: Vec<TransactionTrace> = vec![];

        for broadcasted_transaction in transactions.iter() {
            let blockifier_transaction =
                broadcasted_transaction.to_blockifier_account_transaction(chain_id, true)?;
            let tx_execution_info = blockifier_transaction.execute(
                &mut state.state,
                &self.block_context,
                !skip_fee_charge,
                !skip_validate,
            )?;

            let state_diff: ThinStateDiff = state.extract_state_diff_from_pending_state()?.into();
            let state_diff =
                if state_diff == ThinStateDiff::default() { None } else { Some(state_diff) };

            let address_to_class_hash_map = &state.state.state.address_to_class_hash;

            let validate_invocation =
                if let Some(validate_info) = tx_execution_info.validate_call_info {
                    Some(FunctionInvocation::try_from_call_info(
                        validate_info,
                        address_to_class_hash_map,
                    )?)
                } else {
                    None
                };

            let fee_transfer_invocation =
                if let Some(fee_transfer_info) = tx_execution_info.fee_transfer_call_info {
                    Some(FunctionInvocation::try_from_call_info(
                        fee_transfer_info,
                        address_to_class_hash_map,
                    )?)
                } else {
                    None
                };

            let trace = match broadcasted_transaction {
                BroadcastedTransaction::Declare(_) => {
                    TransactionTrace::Declare(DeclareTransactionTrace {
                        validate_invocation,
                        fee_transfer_invocation,
                        state_diff,
                    })
                }
                BroadcastedTransaction::DeployAccount(_) => {
                    TransactionTrace::DeployAccount(DeployAccountTransactionTrace {
                        validate_invocation,
                        constructor_invocation: if let Some(call_info) =
                            tx_execution_info.execute_call_info
                        {
                            Some(FunctionInvocation::try_from_call_info(
                                call_info,
                                address_to_class_hash_map,
                            )?)
                        } else {
                            None
                        },
                        fee_transfer_invocation,
                        state_diff,
                    })
                }
                BroadcastedTransaction::Invoke(_) => {
                    TransactionTrace::Invoke(InvokeTransactionTrace {
                        fee_transfer_invocation,
                        validate_invocation,
                        state_diff,
                        execute_invocation: match tx_execution_info.execute_call_info {
                            Some(call_info) => match call_info.execution.failed {
                                false => ExecutionInvocation::Succeeded(
                                    FunctionInvocation::try_from_call_info(
                                        call_info,
                                        address_to_class_hash_map,
                                    )?,
                                ),
                                true => ExecutionInvocation::Reverted(
                                    starknet_types::rpc::transactions::Reversion {
                                        revert_reason: tx_execution_info
                                            .revert_error
                                            .unwrap_or("Revert reason not found".into()),
                                    },
                                ),
                            },
                            None => match tx_execution_info.revert_error {
                                Some(revert_reason) => ExecutionInvocation::Reverted(
                                    starknet_types::rpc::transactions::Reversion { revert_reason },
                                ),
                                None => {
                                    return Err(Error::UnexpectedInternalError {
                                        msg: "Simulation contains neither call_info nor \
                                              revert_error"
                                            .into(),
                                    });
                                }
                            },
                        },
                    })
                }
            };

            transactions_traces.push(trace);
        }

        let estimated = estimations::estimate_fee(
            self,
            block_id,
            transactions,
            Some(!skip_fee_charge),
            Some(!skip_validate),
        )?;

        // if the underlying simulation is correct, this should never be the case
        // in alignment with always avoiding assertions in production code, this has to be done
        if transactions_traces.len() != estimated.len() {
            return Err(Error::UnexpectedInternalError {
                msg: format!(
                    "Non-matching number of simulations ({}) and estimations ({})",
                    transactions_traces.len(),
                    estimated.len()
                ),
            });
        }

        let simulation_results = transactions_traces
            .into_iter()
            .zip(estimated)
            .map(|(trace, fee_estimation)| SimulatedTransaction {
                transaction_trace: trace,
                fee_estimation,
            })
            .collect();

        Ok(simulation_results)
    }

    pub fn create_block(&mut self, timestamp: Option<u64>) -> DevnetResult<(), Error> {
        // create new block from pending one
        self.generate_new_block(StateDiff::default(), timestamp)?;
        // clear pending block information
        self.generate_pending_block()?;

        Ok(())
    }

    // Create empty block
    pub fn set_time(&mut self, timestamp: u64) -> DevnetResult<(), Error> {
        self.set_block_timestamp_shift(
            timestamp as i64 - Starknet::get_unix_timestamp_as_seconds() as i64,
        );
        self.create_block(Some(timestamp))
    }

    // Set timestamp shift and create empty block
    pub fn increase_time(&mut self, time_shift: u64) -> DevnetResult<(), Error> {
        self.set_block_timestamp_shift(self.pending_block_timestamp_shift + time_shift as i64);
        self.create_block(None)
    }

    // Set timestamp shift for next blocks
    pub fn set_block_timestamp_shift(&mut self, timestamp: i64) {
        self.pending_block_timestamp_shift = timestamp;
    }

    pub fn get_unix_timestamp_as_seconds() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("should get current UNIX timestamp")
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use blockifier::state::state_api::State;
    use blockifier::transaction::errors::TransactionExecutionError;
    use starknet_api::block::{BlockHash, BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
    use starknet_rs_core::types::{BlockId, BlockTag};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;

    use super::Starknet;
    use crate::blocks::StarknetBlock;
    use crate::constants::{
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_INITIAL_BALANCE, ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::{DevnetResult, Error};
    use crate::starknet::starknet_config::StarknetConfig;
    use crate::state::state_diff::StateDiff;
    use crate::traits::{Accounted, StateChanger, StateExtractor};
    use crate::utils::test_utils::{
        dummy_contract_address, dummy_declare_transaction_v1, dummy_felt,
    };

    #[test]
    fn correct_initial_state_with_test_config() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        let predeployed_accounts = starknet.predeployed_accounts.get_accounts();
        let expected_balance = config.predeployed_accounts_initial_balance;

        for account in predeployed_accounts {
            let account_balance = account.get_balance(&mut starknet.state).unwrap();
            assert_eq!(expected_balance, account_balance);
        }
    }

    #[test]
    fn correct_block_context_creation() {
        let fee_token_address =
            ContractAddress::new(Felt::from_prefixed_hex_str("0xAA").unwrap()).unwrap();
        let block_ctx = Starknet::init_block_context(10, "0xAA", DEVNET_DEFAULT_CHAIN_ID);
        assert_eq!(block_ctx.block_number, BlockNumber(0));
        assert_eq!(block_ctx.block_timestamp, BlockTimestamp(0));
        assert_eq!(block_ctx.gas_prices.eth_l1_gas_price, 10);
        assert_eq!(
            ContractAddress::from(block_ctx.fee_token_addresses.eth_fee_token_address),
            fee_token_address
        );
    }

    #[test]
    fn pending_block_is_correct() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        let initial_block_number = starknet.block_context.block_number;
        starknet.generate_pending_block().unwrap();

        assert_eq!(starknet.pending_block().header.block_number, initial_block_number.next());
    }

    #[test]
    fn correct_new_block_creation() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let tx = dummy_declare_transaction_v1();

        // add transaction hash to pending block
        starknet.blocks.pending_block.add_transaction(tx.transaction_hash);

        // pending block has some transactions
        assert!(!starknet.pending_block().get_transactions().is_empty());
        // blocks collection is empty
        assert!(starknet.blocks.num_to_block.is_empty());

        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        // blocks collection should not be empty
        assert!(!starknet.blocks.num_to_block.is_empty());

        // get block by number and check that the transactions in the block are correct
        let added_block = starknet.blocks.num_to_block.get(&BlockNumber(0)).unwrap();

        assert!(added_block.get_transactions().len() == 1);
        assert_eq!(*added_block.get_transactions().first().unwrap(), tx.transaction_hash);
    }

    #[test]
    fn successful_emptying_of_pending_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let initial_block_number = starknet.block_context.block_number;
        let initial_gas_price = starknet.block_context.gas_prices.eth_l1_gas_price;
        let initial_block_timestamp = starknet.block_context.block_timestamp;
        let initial_sequencer = starknet.block_context.sequencer_address;

        // create pending block with some information in it
        let mut pending_block = StarknetBlock::create_pending_block();
        pending_block.add_transaction(dummy_felt());
        pending_block.status = BlockStatus::AcceptedOnL2;

        // assign the pending block
        starknet.blocks.pending_block = pending_block.clone();
        assert!(*starknet.pending_block() == pending_block);

        // empty the pending to block and check if it is in starting state
        starknet.restart_pending_block().unwrap();

        assert!(*starknet.pending_block() != pending_block);
        assert_eq!(starknet.pending_block().status, BlockStatus::Pending);
        assert!(starknet.pending_block().get_transactions().is_empty());
        assert_eq!(starknet.pending_block().header.timestamp, initial_block_timestamp);
        assert_eq!(starknet.pending_block().header.block_number, initial_block_number);
        assert_eq!(starknet.pending_block().header.parent_hash, BlockHash::default());
        assert_eq!(starknet.pending_block().header.gas_price, GasPrice(initial_gas_price));
        assert_eq!(starknet.pending_block().header.sequencer, initial_sequencer);
    }

    #[test]
    fn correct_block_context_update() {
        let mut block_ctx = Starknet::init_block_context(0, "0x0", DEVNET_DEFAULT_CHAIN_ID);
        let initial_block_number = block_ctx.block_number;
        Starknet::update_block_context(&mut block_ctx);

        assert_eq!(block_ctx.block_number, initial_block_number.next());
    }

    #[test]
    fn getting_state_of_latest_block() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Latest)).expect("Should be OK");
    }

    #[test]
    fn getting_state_of_pending_block() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Pending)).expect("Should be OK");
    }

    #[test]
    fn getting_state_at_block_by_nonexistent_hash() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block(StateDiff::default(), None).unwrap();

        match starknet.get_state_at(&BlockId::Hash(Felt::from(0).into())) {
            Err(Error::NoBlock) => (),
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn getting_nonexistent_state_at_block_by_number() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.blocks.num_to_state.remove(&BlockNumber(0));

        match starknet.get_state_at(&BlockId::Number(0)) {
            Err(Error::NoStateAtBlock { block_number: _ }) => (),
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn calling_method_of_undeployed_contract() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();

        let undeployed_address_hex = "0x1234";
        let undeployed_address = Felt::from_prefixed_hex_str(undeployed_address_hex).unwrap();
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap();

        match starknet.call(
            BlockId::Tag(BlockTag::Latest),
            undeployed_address,
            entry_point_selector.into(),
            vec![],
        ) {
            Err(Error::ContractNotFound) => (),
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    #[test]
    fn calling_nonexistent_contract_method() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();

        let predeployed_account = &starknet.predeployed_accounts.get_accounts()[0];
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("nonExistentMethod").unwrap();

        match starknet.call(
            BlockId::Tag(BlockTag::Latest),
            Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS).unwrap(),
            entry_point_selector.into(),
            vec![Felt::from(predeployed_account.account_address)],
        ) {
            Err(Error::BlockifierTransactionError(
                TransactionExecutionError::EntryPointExecutionError(
                    blockifier::execution::errors::EntryPointExecutionError::PreExecutionError(
                        blockifier::execution::errors::PreExecutionError::EntryPointNotFound(_),
                    ),
                ),
            )) => (),
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    /// utility method for happy path balance retrieval
    fn get_balance_at(
        starknet: &Starknet,
        contract_address: ContractAddress,
    ) -> DevnetResult<Vec<Felt>> {
        let entry_point_selector =
            starknet_rs_core::utils::get_selector_from_name("balanceOf").unwrap();
        starknet.call(
            BlockId::Tag(BlockTag::Latest),
            Felt::from_prefixed_hex_str(ERC20_CONTRACT_ADDRESS)?,
            entry_point_selector.into(),
            vec![Felt::from(contract_address)],
        )
    }

    #[test]
    fn getting_balance_of_predeployed_contract() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();

        let predeployed_account = &starknet.predeployed_accounts.get_accounts()[0];
        let result = get_balance_at(&starknet, predeployed_account.account_address).unwrap();

        let balance_hex = format!("0x{:x}", DEVNET_DEFAULT_INITIAL_BALANCE);
        let balance_felt = Felt::from_prefixed_hex_str(balance_hex.as_str()).unwrap();
        let balance_uint256 = vec![balance_felt, Felt::from_prefixed_hex_str("0x0").unwrap()];
        assert_eq!(result, balance_uint256);
    }

    #[test]
    fn getting_balance_of_undeployed_contract() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();

        let undeployed_address =
            ContractAddress::new(Felt::from_prefixed_hex_str("0x1234").unwrap()).unwrap();
        let result = get_balance_at(&starknet, undeployed_address).unwrap();

        let zero = Felt::from_prefixed_hex_str("0x0").unwrap();
        let expected_balance_uint256 = vec![zero, zero];
        assert_eq!(result, expected_balance_uint256);
    }

    #[test]
    fn correct_latest_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.get_latest_block().err().unwrap();

        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();

        // last added block number -> 0
        let added_block = starknet.blocks.num_to_block.get(&BlockNumber(0)).unwrap();
        // number of the accepted block -> 1
        let block_number = starknet.get_latest_block().unwrap().block_number();

        assert_eq!(block_number.0, added_block.header.block_number.0);

        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();

        let added_block2 = starknet.blocks.num_to_block.get(&BlockNumber(1)).unwrap();
        let block_number2 = starknet.get_latest_block().unwrap().block_number();

        assert_eq!(block_number2.0, added_block2.header.block_number.0);
    }

    #[test]
    fn gets_block_txs_count() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();

        let num_no_transactions = starknet.get_block_txs_count(BlockId::Number(0));

        assert_eq!(num_no_transactions.unwrap(), 0);

        let tx = dummy_declare_transaction_v1();

        // add transaction hash to pending block
        starknet.blocks.pending_block.add_transaction(tx.transaction_hash);

        starknet.generate_new_block(StateDiff::default(), None).unwrap();

        let num_one_transaction = starknet.get_block_txs_count(BlockId::Number(1));

        assert_eq!(num_one_transaction.unwrap(), 1);
    }

    #[test]
    fn returns_chain_id() {
        let config = StarknetConfig::default();
        let starknet = Starknet::new(&config).unwrap();
        let chain_id = starknet.chain_id();

        assert_eq!(chain_id.to_string(), DEVNET_DEFAULT_CHAIN_ID.to_string());
    }

    #[test]
    fn correct_state_at_specific_block() {
        let mut starknet = Starknet::default();
        // generate initial block with empty state
        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();

        // **generate second block**
        // add data to state
        starknet.state.state.increment_nonce(dummy_contract_address().try_into().unwrap()).unwrap();
        // get state difference
        let state_diff = starknet.state.extract_state_diff_from_pending_state().unwrap();
        // move data from pending_state to state
        starknet.state.apply_state_difference(state_diff.clone()).unwrap();
        // generate new block and save the state
        let second_block = starknet.generate_new_block(state_diff, None).unwrap();
        starknet.generate_pending_block().unwrap();

        // **generate third block**
        // add data to state
        starknet.state.state.increment_nonce(dummy_contract_address().try_into().unwrap()).unwrap();
        // get state difference
        let state_diff = starknet.state.extract_state_diff_from_pending_state().unwrap();
        // move data from pending_state to state
        starknet.state.apply_state_difference(state_diff.clone()).unwrap();
        // generate new block and save the state
        let third_block = starknet.generate_new_block(state_diff, None).unwrap();
        starknet.generate_pending_block().unwrap();

        // check modified state at block 1 and 2 to contain the correct value for the nonce
        let second_block_address_nonce = starknet
            .blocks
            .num_to_state
            .get(&second_block)
            .unwrap()
            .state
            .state
            .address_to_nonce
            .get(&dummy_contract_address())
            .unwrap();
        let second_block_expected_address_nonce = Felt::from(1);
        assert_eq!(second_block_expected_address_nonce, *second_block_address_nonce);

        let third_block_address_nonce = starknet
            .blocks
            .num_to_state
            .get(&third_block)
            .unwrap()
            .state
            .state
            .address_to_nonce
            .get(&dummy_contract_address())
            .unwrap();
        let third_block_expected_address_nonce = Felt::from(2);
        assert_eq!(third_block_expected_address_nonce, *third_block_address_nonce);
    }

    #[test]
    fn gets_latest_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();
        starknet.generate_new_block(StateDiff::default(), None).unwrap();
        starknet.generate_pending_block().unwrap();
        starknet.generate_new_block(StateDiff::default(), None).unwrap();

        let latest_block = starknet.get_latest_block();

        assert_eq!(latest_block.unwrap().block_number(), BlockNumber(2));
    }
    #[test]
    fn check_timestamp_of_newly_generated_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        Starknet::update_block_context(&mut starknet.block_context);
        starknet.generate_pending_block().unwrap();
        starknet
            .blocks
            .pending_block
            .set_timestamp(BlockTimestamp(Starknet::get_unix_timestamp_as_seconds()));
        let pending_block_timestamp = starknet.pending_block().header.timestamp;

        let sleep_duration_secs = 5;
        thread::sleep(Duration::from_secs(sleep_duration_secs));
        starknet.generate_new_block(StateDiff::default(), None).unwrap();

        let block_timestamp = starknet.get_latest_block().unwrap().header.timestamp;
        // check if the pending_block_timestamp is less than the block_timestamp,
        // by number of sleep seconds because the timeline of events is this:
        // ----(pending block timestamp)----(sleep)----(new block timestamp)
        assert!(pending_block_timestamp.0 + sleep_duration_secs <= block_timestamp.0);
    }
}
