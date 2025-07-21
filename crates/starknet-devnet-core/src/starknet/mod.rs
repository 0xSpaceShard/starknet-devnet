use std::num::NonZeroU128;
use std::sync::Arc;

use blockifier::context::{BlockContext, ChainInfo, TransactionContext};
use blockifier::execution::common_hints::ExecutionMode;
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::{AccountTransaction, ExecutionFlags};
use blockifier::transaction::errors::TransactionExecutionError;
use blockifier::transaction::objects::TransactionExecutionInfo;
use blockifier::transaction::transactions::ExecutableTransaction;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use ethers::types::H256;
use parking_lot::RwLock;
use starknet_api::block::{
    BlockInfo, BlockNumber, BlockTimestamp, FeeType, GasPrice, GasPricePerToken, GasPriceVector,
    GasPrices,
};
use starknet_api::core::SequencerContractAddress;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::fields::{GasVectorComputationMode, Tip};
use starknet_api::transaction::{TransactionHasher, TransactionVersion};
use starknet_rs_core::types::{BlockId, BlockTag, Felt, Hash256, MsgFromL1};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_signers::{LocalWallet, Signer, SigningKey};
use starknet_types::chain_id::ChainId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::emitted_event::EmittedEvent;
use starknet_types::felt::{
    BlockHash, ClassHash, TransactionHash, felt_from_prefixed_hex, split_biguint,
};
use starknet_types::num_bigint::BigUint;
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::rpc::block::{
    Block, BlockHeader, BlockId as CustomBlockId, BlockResult, BlockStatus, PreConfirmedBlock,
    PreConfirmedBlockHeader,
};
use starknet_types::rpc::estimate_message_fee::FeeEstimateWrapper;
use starknet_types::rpc::gas_modification::{GasModification, GasModificationRequest};
use starknet_types::rpc::state::{
    PreConfirmedStateUpdate, StateUpdate, StateUpdateResult, ThinStateDiff,
};
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;
use starknet_types::rpc::transactions::{
    BlockTransactionTrace, BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, BroadcastedTransaction, BroadcastedTransactionCommonV3,
    L1HandlerTransactionStatus, ResourceBoundsWrapper, SimulatedTransaction, SimulationFlag,
    TransactionFinalityStatus, TransactionStatus, TransactionTrace, TransactionType,
    TransactionWithHash, TransactionWithReceipt, Transactions,
};
use starknet_types::traits::HashProducer;
use tracing::{error, info};

use self::cheats::Cheats;
use self::defaulter::StarknetDefaulter;
use self::predeployed::initialize_erc20_at_address;
use self::starknet_config::{StarknetConfig, StateArchiveCapacity};
use self::transaction_trace::create_trace;
use crate::account::Account;
use crate::blocks::{StarknetBlock, StarknetBlocks};
use crate::constants::{
    ARGENT_CONTRACT_CLASS_HASH, ARGENT_CONTRACT_SIERRA, ARGENT_MULTISIG_CONTRACT_CLASS_HASH,
    ARGENT_MULTISIG_CONTRACT_SIERRA, CHARGEABLE_ACCOUNT_ADDRESS, CHARGEABLE_ACCOUNT_PRIVATE_KEY,
    DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_L1_DATA_GAS_PRICE, DEVNET_DEFAULT_L1_GAS_PRICE,
    DEVNET_DEFAULT_L2_GAS_PRICE, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
    ENTRYPOINT_NOT_FOUND_ERROR_ENCODED, ETH_ERC20_CONTRACT_ADDRESS, ETH_ERC20_NAME,
    ETH_ERC20_SYMBOL, STRK_ERC20_CONTRACT_ADDRESS, STRK_ERC20_NAME, STRK_ERC20_SYMBOL, USE_KZG_DA,
};
use crate::contract_class_choice::AccountContractClassChoice;
use crate::error::{ContractExecutionError, DevnetResult, Error, TransactionValidationError};
use crate::messaging::MessagingBroker;
use crate::nonzero_gas_price;
use crate::predeployed_accounts::PredeployedAccounts;
use crate::state::state_diff::StateDiff;
use crate::state::{CommittedClassStorage, CustomState, CustomStateReader, StarknetState};
use crate::traits::{AccountGenerator, Deployed, HashIdentified, HashIdentifiedMut};
use crate::transactions::{StarknetTransaction, StarknetTransactions};
use crate::utils::{custom_bouncer_config, get_versioned_constants, maybe_extract_failure_reason};

mod add_declare_transaction;
mod add_deploy_account_transaction;
mod add_invoke_transaction;
mod add_l1_handler_transaction;
mod cheats;
pub(crate) mod defaulter;
mod estimations;
pub mod events;
mod get_class_impls;
mod predeployed;
pub mod starknet_config;
mod state_update;
pub(crate) mod transaction_trace;

pub struct Starknet {
    pub latest_state: StarknetState,
    pub pre_confirmed_state: StarknetState,
    /// Contains the diff since the last block
    pre_confirmed_state_diff: StateDiff,
    predeployed_accounts: PredeployedAccounts,
    pub(in crate::starknet) block_context: BlockContext,
    // To avoid repeating some logic related to blocks,
    // having `blocks` public allows to re-use functions like `get_blocks()`.
    pub(crate) blocks: StarknetBlocks,
    pub transactions: StarknetTransactions,
    pub config: StarknetConfig,
    pub pre_confirmed_block_timestamp_shift: i64,
    pub next_block_timestamp: Option<u64>,
    pub next_block_gas: GasModification,
    pub(crate) messaging: MessagingBroker,
    rpc_contract_classes: Arc<RwLock<CommittedClassStorage>>,
    cheats: Cheats,
}

impl Default for Starknet {
    fn default() -> Self {
        Self {
            block_context: Self::init_block_context(
                DEVNET_DEFAULT_L1_GAS_PRICE,
                DEVNET_DEFAULT_L1_GAS_PRICE,
                DEVNET_DEFAULT_L1_DATA_GAS_PRICE,
                DEVNET_DEFAULT_L1_DATA_GAS_PRICE,
                DEVNET_DEFAULT_L2_GAS_PRICE,
                DEVNET_DEFAULT_L2_GAS_PRICE,
                ETH_ERC20_CONTRACT_ADDRESS,
                STRK_ERC20_CONTRACT_ADDRESS,
                DEVNET_DEFAULT_CHAIN_ID,
                DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
            ),
            latest_state: Default::default(),
            pre_confirmed_state: Default::default(),
            pre_confirmed_state_diff: Default::default(),
            predeployed_accounts: Default::default(),
            blocks: Default::default(),
            transactions: Default::default(),
            config: Default::default(),
            pre_confirmed_block_timestamp_shift: 0,
            next_block_timestamp: None,
            next_block_gas: GasModification {
                gas_price_wei: DEVNET_DEFAULT_L1_GAS_PRICE,
                data_gas_price_wei: DEVNET_DEFAULT_L1_DATA_GAS_PRICE,
                gas_price_fri: DEVNET_DEFAULT_L1_GAS_PRICE,
                data_gas_price_fri: DEVNET_DEFAULT_L1_DATA_GAS_PRICE,
                l2_gas_price_fri: DEVNET_DEFAULT_L2_GAS_PRICE,
                l2_gas_price_wei: DEVNET_DEFAULT_L2_GAS_PRICE,
            },
            messaging: Default::default(),
            rpc_contract_classes: Default::default(),
            cheats: Default::default(),
        }
    }
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let defaulter = StarknetDefaulter::new(config.fork_config.clone());
        let rpc_contract_classes = Arc::new(RwLock::new(CommittedClassStorage::default()));
        let mut state = StarknetState::new(defaulter, rpc_contract_classes.clone());

        // predeclare account classes eligible for predeployment
        for account_class_choice in
            [AccountContractClassChoice::Cairo0, AccountContractClassChoice::Cairo1]
        {
            let class_wrapper = account_class_choice.get_class_wrapper()?;
            state.predeclare_contract_class(
                class_wrapper.class_hash,
                class_wrapper.contract_class,
            )?;
        }

        // predeclare argent account classes (not predeployable)
        if config.predeclare_argent {
            for (class_hash, raw_sierra) in [
                (ARGENT_CONTRACT_CLASS_HASH, ARGENT_CONTRACT_SIERRA),
                (ARGENT_MULTISIG_CONTRACT_CLASS_HASH, ARGENT_MULTISIG_CONTRACT_SIERRA),
            ] {
                let contract_class =
                    ContractClass::Cairo1(ContractClass::cairo_1_from_sierra_json_str(raw_sierra)?);
                state.predeclare_contract_class(class_hash, contract_class)?;
            }
        }

        // deploy udc, eth erc20 and strk erc20 contracts
        let eth_erc20_fee_contract = predeployed::create_erc20_at_address_extended(
            ETH_ERC20_CONTRACT_ADDRESS,
            config.eth_erc20_class_hash,
            &config.eth_erc20_contract_class,
        )?;
        let strk_erc20_fee_contract = predeployed::create_erc20_at_address_extended(
            STRK_ERC20_CONTRACT_ADDRESS,
            config.strk_erc20_class_hash,
            &config.strk_erc20_contract_class,
        )?;

        for udc in [predeployed::create_legacy_udc()?, predeployed::create_udc()?] {
            udc.deploy(&mut state)?;
        }

        eth_erc20_fee_contract.deploy(&mut state)?;
        initialize_erc20_at_address(
            &mut state,
            ETH_ERC20_CONTRACT_ADDRESS,
            ETH_ERC20_NAME,
            ETH_ERC20_SYMBOL,
        )?;

        strk_erc20_fee_contract.deploy(&mut state)?;
        initialize_erc20_at_address(
            &mut state,
            STRK_ERC20_CONTRACT_ADDRESS,
            STRK_ERC20_NAME,
            STRK_ERC20_SYMBOL,
        )?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance.clone(),
            eth_erc20_fee_contract.get_address(),
            strk_erc20_fee_contract.get_address(),
        );

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            config.account_contract_class_hash,
            &config.account_contract_class,
        )?;
        for account in accounts {
            account.deploy(&mut state)?;
        }

        let chargeable_account = Account::new_chargeable(
            eth_erc20_fee_contract.get_address(),
            strk_erc20_fee_contract.get_address(),
        )?;

        if state
            .state
            .get_class_hash_at(chargeable_account.account_address.into())
            .is_ok_and(|h| h.0 == Felt::ZERO)
        {
            chargeable_account.deploy(&mut state)?;
        }

        // when forking, the number of the first new block to be mined is equal to the last origin
        // block (the one specified by the user) plus one.
        // The parent hash of the first new block is equal to the last origin block hash.
        let starting_block_number =
            config.fork_config.block_number.map_or(DEVNET_DEFAULT_STARTING_BLOCK_NUMBER, |n| n + 1);
        let last_block_hash = config.fork_config.block_hash;

        let pre_confirmed_state_diff = state.commit_diff(starting_block_number)?;

        let mut this = Self {
            latest_state: Default::default(), // temporary - overwritten on genesis block creation
            pre_confirmed_state: state,
            pre_confirmed_state_diff,
            predeployed_accounts,
            block_context: Self::init_block_context(
                config.gas_price_wei,
                config.gas_price_fri,
                config.data_gas_price_wei,
                config.data_gas_price_fri,
                config.l2_gas_price_wei,
                config.l2_gas_price_fri,
                ETH_ERC20_CONTRACT_ADDRESS,
                STRK_ERC20_CONTRACT_ADDRESS,
                config.chain_id,
                starting_block_number,
            ),
            blocks: StarknetBlocks::new(starting_block_number, last_block_hash),
            transactions: StarknetTransactions::default(),
            config: config.clone(),
            pre_confirmed_block_timestamp_shift: 0,
            next_block_timestamp: None,
            next_block_gas: GasModification {
                gas_price_wei: config.gas_price_wei,
                data_gas_price_wei: config.data_gas_price_wei,
                gas_price_fri: config.gas_price_fri,
                data_gas_price_fri: config.data_gas_price_fri,
                l2_gas_price_wei: config.l2_gas_price_wei,
                l2_gas_price_fri: config.l2_gas_price_fri,
            },
            messaging: Default::default(),
            rpc_contract_classes,
            cheats: Default::default(),
        };

        this.restart_pre_confirmed_block()?;

        // Create an empty genesis block, set start_time before if it's set
        if let Some(start_time) = config.start_time {
            this.set_next_block_timestamp(start_time);
        };
        this.create_block()?;

        Ok(this)
    }

    pub fn get_state(&mut self) -> &mut StarknetState {
        &mut self.pre_confirmed_state
    }

    pub fn restart(&mut self, restart_l1_to_l2_messaging: bool) -> DevnetResult<()> {
        let new_messsaging_ethereum =
            if restart_l1_to_l2_messaging { None } else { self.messaging.ethereum.clone() };

        *self = Starknet::new(&self.config)?;
        self.messaging.ethereum = new_messsaging_ethereum;

        info!("Starknet Devnet restarted");
        Ok(())
    }

    pub fn get_predeployed_accounts(&self) -> Vec<Account> {
        self.predeployed_accounts.get_accounts().to_vec()
    }

    // Update block context
    // Initialize values for new pre_confirmed block
    pub(crate) fn generate_pre_confirmed_block(&mut self) -> DevnetResult<()> {
        Self::advance_block_context_block_number(&mut self.block_context);

        Self::set_block_context_gas(&mut self.block_context, &self.next_block_gas);

        // Pre_confirmed block header gas data needs to be set
        let header = &mut self.blocks.pre_confirmed_block.header.block_header_without_hash;
        header.l1_gas_price.price_in_wei = GasPrice(self.next_block_gas.gas_price_wei.get());
        header.l1_data_gas_price.price_in_wei =
            GasPrice(self.next_block_gas.data_gas_price_wei.get());
        header.l2_gas_price.price_in_wei = GasPrice(self.next_block_gas.l2_gas_price_wei.get());
        header.l1_gas_price.price_in_fri = GasPrice(self.next_block_gas.gas_price_fri.get());
        header.l1_data_gas_price.price_in_fri =
            GasPrice(self.next_block_gas.data_gas_price_fri.get());
        header.l2_gas_price.price_in_fri = GasPrice(self.next_block_gas.l2_gas_price_fri.get());

        self.restart_pre_confirmed_block()?;

        Ok(())
    }

    fn next_block_timestamp(&mut self) -> BlockTimestamp {
        match self.next_block_timestamp {
            Some(timestamp) => {
                self.next_block_timestamp = None;
                BlockTimestamp(timestamp)
            }
            None => BlockTimestamp(
                (Starknet::get_unix_timestamp_as_seconds() as i64
                    + self.pre_confirmed_block_timestamp_shift) as u64,
            ),
        }
    }

    /// Transfer data from pre_confirmed block into new block and save it to blocks collection.
    /// Generates new pre_confirmed block. Same for pre_confirmed state. Returns the new block hash.
    pub(crate) fn generate_new_block_and_state(&mut self) -> DevnetResult<Felt> {
        let mut new_block = self.pre_confirmed_block().clone();

        // Set new block header
        // TODO why not store the whole next block header instead of storing separate properties?
        new_block.header.block_header_without_hash.l1_gas_price.price_in_fri =
            GasPrice(self.next_block_gas.gas_price_fri.get());
        new_block.header.block_header_without_hash.l1_gas_price.price_in_wei =
            GasPrice(self.next_block_gas.gas_price_wei.get());
        new_block.header.block_header_without_hash.l1_data_gas_price.price_in_fri =
            GasPrice(self.next_block_gas.data_gas_price_fri.get());
        new_block.header.block_header_without_hash.l1_data_gas_price.price_in_wei =
            GasPrice(self.next_block_gas.data_gas_price_wei.get());
        new_block.header.block_header_without_hash.l2_gas_price.price_in_fri =
            GasPrice(self.next_block_gas.l2_gas_price_fri.get());
        new_block.header.block_header_without_hash.l2_gas_price.price_in_wei =
            GasPrice(self.next_block_gas.l2_gas_price_wei.get());

        let new_block_number = self.blocks.next_latest_block_number();
        new_block.set_block_hash(if self.config.lite_mode {
            BlockHash::from(new_block_number.0)
        } else {
            new_block.generate_hash()?
        });
        new_block.status = BlockStatus::AcceptedOnL2;
        new_block.set_block_number(new_block_number.0);

        // set block timestamp and context block timestamp for contract execution
        let block_timestamp = self.next_block_timestamp();
        new_block.set_timestamp(block_timestamp);
        Self::update_block_context_block_timestamp(&mut self.block_context, block_timestamp);

        let new_block_hash = new_block.header.block_hash.0;

        // update txs block hash block number for each transaction in the pre_confirmed block
        new_block.get_transactions().iter().for_each(|tx_hash| {
            if let Some(tx) = self.transactions.get_by_hash_mut(tx_hash) {
                tx.block_hash = Some(new_block_hash);
                tx.block_number = Some(new_block_number);
                tx.finality_status = TransactionFinalityStatus::AcceptedOnL2;
            } else {
                error!("Transaction is not present in the transactions collection");
            }
        });

        // insert pre_confirmed block in the blocks collection and connect it to the state diff
        self.blocks.insert(new_block, self.pre_confirmed_state_diff.clone());
        self.pre_confirmed_state_diff = StateDiff::default();

        // save into blocks state archive
        if self.config.state_archive == StateArchiveCapacity::Full {
            let clone = self.pre_confirmed_state.clone_historic();
            self.blocks.save_state_at(new_block_hash, clone);
        }

        self.generate_pre_confirmed_block()?;

        // for every new block we need to clone pre_confirmed state into state
        self.latest_state = self.pre_confirmed_state.clone_historic();

        Ok(new_block_hash)
    }

    /// Commits the changes since the last commit. Use it to commit the changes generated by the
    /// last tx. Updates the `pre_confirmed_state_diff` to accumulate the changes since the last
    /// block. Check `StarknetState::commit_diff` for more info.
    pub fn commit_diff(&mut self) -> DevnetResult<StateDiff> {
        let next_block_number = self.blocks.next_latest_block_number();
        let state_diff = self.pre_confirmed_state.commit_diff(next_block_number.0)?;
        self.pre_confirmed_state_diff.extend(&state_diff);

        Ok(state_diff)
    }

    /// Handles succeeded and reverted transactions. The tx is stored and potentially dumped. A new
    /// block is generated in block-generation-on-transaction mode.
    pub(crate) fn handle_accepted_transaction(
        &mut self,
        transaction: TransactionWithHash,
        tx_info: TransactionExecutionInfo,
    ) -> DevnetResult<()> {
        let state_diff = self.commit_diff()?;
        let transaction_hash = transaction.get_transaction_hash();
        let gas_vector_computation_mode = transaction.transaction.gas_vector_computation_mode();

        let trace = create_trace(
            &mut self.pre_confirmed_state.state,
            transaction.get_type(),
            &tx_info,
            state_diff.into(),
            self.block_context.versioned_constants(),
            &gas_vector_computation_mode,
        )?;
        let transaction_to_add = StarknetTransaction::pre_confirm(&transaction, tx_info, trace);

        // add accepted transaction to pre_confirmed block
        self.blocks.pre_confirmed_block.add_transaction(*transaction_hash);

        self.transactions.insert(transaction_hash, transaction_to_add);

        // create new block from pre_confirmed one, only in block-generation-on-transaction mode
        if !self.config.uses_pre_confirmed_block() {
            self.generate_new_block_and_state()?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    /// Create a BlockContext based on BlockContext::create_for_testing()
    fn init_block_context(
        gas_price_wei: NonZeroU128,
        gas_price_fri: NonZeroU128,
        data_gas_price_wei: NonZeroU128,
        data_gas_price_fri: NonZeroU128,
        l2_gas_price_wei: NonZeroU128,
        l2_gas_price_fri: NonZeroU128,
        eth_fee_token_address: Felt,
        strk_fee_token_address: Felt,
        chain_id: ChainId,
        block_number: u64,
    ) -> BlockContext {
        let block_info = BlockInfo {
            block_number: BlockNumber(block_number),
            block_timestamp: BlockTimestamp(0),
            sequencer_address: starknet_api::contract_address!("0x1000"),
            gas_prices: GasPrices {
                eth_gas_prices: GasPriceVector {
                    l1_gas_price: nonzero_gas_price!(gas_price_wei),
                    l1_data_gas_price: nonzero_gas_price!(data_gas_price_wei),
                    l2_gas_price: nonzero_gas_price!(l2_gas_price_wei),
                },
                strk_gas_prices: GasPriceVector {
                    l1_gas_price: nonzero_gas_price!(gas_price_fri),
                    l1_data_gas_price: nonzero_gas_price!(data_gas_price_fri),
                    l2_gas_price: nonzero_gas_price!(l2_gas_price_fri),
                },
            },
            use_kzg_da: USE_KZG_DA,
        };

        let chain_info = ChainInfo {
            chain_id: chain_id.into(),
            fee_token_addresses: blockifier::context::FeeTokenAddresses {
                eth_fee_token_address: starknet_api::contract_address!(
                    eth_fee_token_address.to_hex_string().as_str()
                ),
                strk_fee_token_address: starknet_api::contract_address!(
                    strk_fee_token_address.to_hex_string().as_str()
                ),
            },
        };

        BlockContext::new(
            block_info,
            chain_info,
            get_versioned_constants(),
            custom_bouncer_config(),
        )
    }

    /// Update block context block_number with the next one
    /// # Arguments
    /// * `block_context` - BlockContext to be updated
    fn advance_block_context_block_number(block_context: &mut BlockContext) {
        let mut block_info = block_context.block_info().clone();
        block_info.block_number = block_info.block_number.next().unwrap_or_default();
        // TODO: update block_context via preferred method in the documentation
        *block_context = BlockContext::new(
            block_info,
            block_context.chain_info().clone(),
            get_versioned_constants(),
            custom_bouncer_config(),
        );
    }

    fn set_block_number(&mut self, block_number: u64) {
        self.blocks.pre_confirmed_block.set_block_number(block_number);

        let mut block_info = self.block_context.block_info().clone();
        block_info.block_number.0 = block_number;

        // TODO: update block_context via preferred method in the documentation
        self.block_context = BlockContext::new(
            block_info,
            self.block_context.chain_info().clone(),
            get_versioned_constants(),
            custom_bouncer_config(),
        );
    }

    fn set_block_context_gas(block_context: &mut BlockContext, gas_modification: &GasModification) {
        let mut block_info = block_context.block_info().clone();

        // Block info gas needs to be set here
        block_info.gas_prices = GasPrices {
            eth_gas_prices: GasPriceVector {
                l1_gas_price: nonzero_gas_price!(gas_modification.gas_price_wei),
                l1_data_gas_price: nonzero_gas_price!(gas_modification.data_gas_price_wei),
                l2_gas_price: nonzero_gas_price!(gas_modification.l2_gas_price_wei),
            },
            strk_gas_prices: GasPriceVector {
                l1_gas_price: nonzero_gas_price!(gas_modification.gas_price_fri),
                l1_data_gas_price: nonzero_gas_price!(gas_modification.data_gas_price_fri),
                l2_gas_price: nonzero_gas_price!(gas_modification.l2_gas_price_fri),
            },
        };

        // TODO: update block_context via preferred method in the documentation
        *block_context = BlockContext::new(
            block_info,
            block_context.chain_info().clone(),
            get_versioned_constants(),
            custom_bouncer_config(),
        );
    }

    fn update_block_context_block_timestamp(
        block_context: &mut BlockContext,
        block_timestamp: BlockTimestamp,
    ) {
        let mut block_info = block_context.block_info().clone();
        block_info.block_timestamp = block_timestamp;

        // TODO: update block_context via preferred method in the documentation
        *block_context = BlockContext::new(
            block_info,
            block_context.chain_info().clone(),
            get_versioned_constants(),
            custom_bouncer_config(),
        );
    }

    fn pre_confirmed_block(&self) -> &StarknetBlock {
        &self.blocks.pre_confirmed_block
    }

    /// Restarts pre-confirmed block with information from block_context
    pub(crate) fn restart_pre_confirmed_block(&mut self) -> DevnetResult<()> {
        let mut block = StarknetBlock::create_pre_confirmed_block();

        block.header.block_header_without_hash.block_number =
            self.block_context.block_info().block_number;

        let gas_prices = &self.block_context.block_info().gas_prices;
        block.header.block_header_without_hash.l1_gas_price = GasPricePerToken {
            price_in_fri: gas_prices.l1_gas_price(&FeeType::Strk).get(),
            price_in_wei: gas_prices.l1_gas_price(&FeeType::Eth).get(),
        };
        block.header.block_header_without_hash.l1_data_gas_price = GasPricePerToken {
            price_in_fri: gas_prices.l1_data_gas_price(&FeeType::Strk).get(),
            price_in_wei: gas_prices.l1_data_gas_price(&FeeType::Eth).get(),
        };
        block.header.block_header_without_hash.l2_gas_price = GasPricePerToken {
            price_in_fri: gas_prices.l2_gas_price(&FeeType::Strk).get(),
            price_in_wei: gas_prices.l2_gas_price(&FeeType::Eth).get(),
        };

        block.header.block_header_without_hash.sequencer =
            SequencerContractAddress(self.block_context.block_info().sequencer_address);

        block.set_timestamp(self.block_context.block_info().block_timestamp);

        self.blocks.pre_confirmed_block = block;

        Ok(())
    }

    fn get_mut_state_at(&mut self, block_id: &BlockId) -> DevnetResult<&mut StarknetState> {
        match block_id {
            BlockId::Tag(BlockTag::Latest) => Ok(&mut self.latest_state),
            BlockId::Tag(BlockTag::PreConfirmed) => Ok(&mut self.pre_confirmed_state),
            _ => {
                let block = self.get_block(block_id)?;
                let block_hash = block.block_hash();

                if self.blocks.last_block_hash == Some(block_hash) {
                    return Ok(&mut self.latest_state);
                }

                if self.config.state_archive == StateArchiveCapacity::None {
                    return Err(Error::NoStateAtBlock { block_id: *block_id });
                }

                let state = self
                    .blocks
                    .hash_to_state
                    .get_mut(&block_hash)
                    .ok_or(Error::NoStateAtBlock { block_id: *block_id })?;
                Ok(state)
            }
        }
    }

    pub fn get_class_hash_at(
        &mut self,
        block_id: &BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<ClassHash> {
        get_class_impls::get_class_hash_at_impl(self, block_id, contract_address)
    }

    pub fn get_class(
        &self,
        block_id: &BlockId,
        class_hash: ClassHash,
    ) -> DevnetResult<ContractClass> {
        get_class_impls::get_class_impl(self, block_id, class_hash)
    }

    pub fn get_class_at(
        &mut self,
        block_id: &BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<ContractClass> {
        get_class_impls::get_class_at_impl(self, block_id, contract_address)
    }

    pub fn get_compiled_casm(&self, class_hash: ClassHash) -> DevnetResult<CasmContractClass> {
        get_class_impls::get_compiled_casm_impl(self, class_hash)
    }

    pub fn call(
        &mut self,
        block_id: &BlockId,
        contract_address: Felt,
        entrypoint_selector: Felt,
        calldata: Vec<Felt>,
    ) -> DevnetResult<Vec<Felt>> {
        let block_context = Arc::new(self.block_context.clone());
        let state = self.get_mut_state_at(block_id)?;

        state.assert_contract_deployed(ContractAddress::new(contract_address)?)?;
        let storage_address = contract_address.try_into()?;
        let class_hash = state.get_class_hash_at(storage_address)?;

        let mut initial_gas =
            block_context.versioned_constants().sierra_gas_limit(&ExecutionMode::Execute);
        let call = blockifier::execution::entry_point::CallEntryPoint {
            calldata: starknet_api::transaction::fields::Calldata(std::sync::Arc::new(
                calldata.clone(),
            )),
            storage_address: contract_address.try_into()?,
            entry_point_selector: starknet_api::core::EntryPointSelector(entrypoint_selector),
            initial_gas: initial_gas.0,
            class_hash: Some(class_hash),
            ..Default::default()
        };

        let mut execution_context =
            blockifier::execution::entry_point::EntryPointExecutionContext::new(
                Arc::new(TransactionContext {
                    block_context: block_context.clone(),
                    tx_info: blockifier::transaction::objects::TransactionInfo::Deprecated(
                        blockifier::transaction::objects::DeprecatedTransactionInfo::default(),
                    ),
                }),
                blockifier::execution::common_hints::ExecutionMode::Execute,
                false,
                blockifier::execution::entry_point::SierraGasRevertTracker::new(initial_gas),
            );

        let mut transactional_state = CachedState::create_transactional(&mut state.state);
        let res = call
            .execute(&mut transactional_state, &mut execution_context, &mut initial_gas.0)
            .map_err(|error| {
                Error::ContractExecutionError(
                    TransactionExecutionError::ExecutionError {
                        error,
                        class_hash,
                        storage_address,
                        selector: starknet_api::core::EntryPointSelector(entrypoint_selector),
                    }
                    .into(),
                )
            })?;

        if res.execution.failed {
            if res.execution.retdata.0.first() == Some(&ENTRYPOINT_NOT_FOUND_ERROR_ENCODED) {
                return Err(Error::EntrypointNotFound);
            } else {
                return Err(Error::ContractExecutionError(ContractExecutionError::from(&res)));
            }
        }

        Ok(res.execution.retdata.0)
    }

    pub fn estimate_fee(
        &mut self,
        block_id: &BlockId,
        transactions: &[BroadcastedTransaction],
        simulation_flags: &[SimulationFlag],
    ) -> DevnetResult<Vec<FeeEstimateWrapper>> {
        let mut skip_validate = false;
        for flag in simulation_flags.iter() {
            if *flag == SimulationFlag::SkipValidate {
                skip_validate = true;
            }
        }
        estimations::estimate_fee(self, block_id, transactions, None, Some(!skip_validate), true)
    }

    pub fn estimate_message_fee(
        &mut self,
        block_id: &CustomBlockId,
        message: MsgFromL1,
    ) -> DevnetResult<FeeEstimateWrapper> {
        estimations::estimate_message_fee(self, block_id, message)
    }

    pub fn add_declare_transaction(
        &mut self,
        declare_transaction: BroadcastedDeclareTransaction,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        add_declare_transaction::add_declare_transaction(self, declare_transaction)
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

    pub fn add_l1_handler_transaction(
        &mut self,
        l1_handler_transaction: L1HandlerTransaction,
    ) -> DevnetResult<TransactionHash> {
        add_l1_handler_transaction::add_l1_handler_transaction(self, l1_handler_transaction)
    }

    fn minting_calldata(
        fundable_address: ContractAddress,
        amount: BigUint,
        erc20_address: ContractAddress,
    ) -> DevnetResult<Vec<Felt>> {
        let (high, low) = split_biguint(amount);

        let mut calldata = vec![
            Felt::ONE,            // number of calls
            erc20_address.into(), // target address
            get_selector_from_name("permissioned_mint")
                .map_err(|e| Error::UnexpectedInternalError { msg: e.to_string() })?,
        ];

        let raw_calldata = vec![Felt::from(fundable_address), low, high];
        calldata.push(raw_calldata.len().into());
        for el in raw_calldata {
            calldata.push(el);
        }

        Ok(calldata)
    }

    /// Creates an invoke tx for minting, using the chargeable account.
    /// Uses permissioned_mint function of the ERC20 contract
    pub async fn mint(
        &mut self,
        fundable_address: ContractAddress,
        amount: BigUint,
        erc20_address: ContractAddress,
    ) -> DevnetResult<Felt> {
        let chargeable_address = felt_from_prefixed_hex(CHARGEABLE_ACCOUNT_ADDRESS)?;
        let state = self.get_state();
        let nonce = state
            .get_nonce_at(starknet_api::core::ContractAddress::try_from(chargeable_address)?)?;

        let unsigned_tx = BroadcastedInvokeTransactionV3 {
            sender_address: ContractAddress::new(chargeable_address)?,
            calldata: Self::minting_calldata(fundable_address, amount, erc20_address)?,
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce: nonce.0,
                resource_bounds: ResourceBoundsWrapper::new(
                    1_000_000,
                    self.config.gas_price_fri.get(),
                    1_000_000,
                    self.config.data_gas_price_fri.get(),
                    1_000_000_000,
                    self.config.l2_gas_price_fri.get(),
                ),
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            },
            account_deployment_data: vec![],
        };

        // generate signature by signing the tx hash
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(felt_from_prefixed_hex(
            CHARGEABLE_ACCOUNT_PRIVATE_KEY,
        )?));
        let tx_hash = unsigned_tx
            .create_sn_api_invoke()?
            .calculate_transaction_hash(&self.config.chain_id.into(), &TransactionVersion::THREE)?;
        let signature = signer.sign_hash(&tx_hash).await?;

        let mut invoke_tx = unsigned_tx;
        invoke_tx.common.signature = vec![signature.r, signature.s];

        // apply the invoke tx
        add_invoke_transaction::add_invoke_transaction(
            self,
            BroadcastedInvokeTransaction::V3(invoke_tx),
        )
    }

    pub fn block_state_update(&self, block_id: &BlockId) -> DevnetResult<StateUpdateResult> {
        let state_update = state_update::state_update_by_block_id(self, block_id)?;

        // StateUpdate needs to be mapped to PreConfirmedStateUpdate when block_id is pre_confirmed
        if block_id == &BlockId::Tag(BlockTag::PreConfirmed) {
            Ok(StateUpdateResult::PreConfirmedStateUpdate(PreConfirmedStateUpdate {
                old_root: state_update.old_root,
                state_diff: state_update.state_diff,
            }))
        } else {
            Ok(StateUpdateResult::StateUpdate(StateUpdate {
                block_hash: state_update.block_hash,
                new_root: state_update.new_root,
                old_root: state_update.old_root,
                state_diff: state_update.state_diff,
            }))
        }
    }

    pub fn set_next_block_gas(
        &mut self,
        gas_prices: GasModificationRequest,
    ) -> DevnetResult<GasModification> {
        self.next_block_gas.update(gas_prices.clone());

        // If generate_block is true, generate new block, for now custom dump_event is None but in
        // future it will change to GasSetEvent with self.next_block_gas data
        if let Some(true) = gas_prices.generate_block {
            self.create_block()?
        }

        Ok(self.next_block_gas.clone())
    }

    pub fn abort_blocks(
        &mut self,
        mut starting_block_id: BlockId,
    ) -> DevnetResult<Vec<TransactionHash>> {
        if self.config.state_archive != StateArchiveCapacity::Full {
            let msg = "The abort blocks feature requires state-archive-capacity set to full.";
            return Err(Error::UnsupportedAction { msg: msg.into() });
        }

        if starting_block_id == BlockId::Tag(BlockTag::PreConfirmed) {
            self.create_block()?;
            starting_block_id = BlockId::Tag(BlockTag::Latest);
        }

        let starting_block_hash = match self.blocks.get_by_block_id(&starting_block_id) {
            Some(block) => block.block_hash(),
            None => return Err(Error::NoBlock),
        };

        let genesis_block = self
            .blocks
            .get_by_block_id(&BlockId::Number(self.blocks.starting_block_number))
            .ok_or(Error::UnsupportedAction { msg: "Cannot abort - no genesis block".into() })?;

        if starting_block_hash == genesis_block.block_hash() {
            return Err(Error::UnsupportedAction { msg: "Genesis block can't be aborted".into() });
        }

        let mut next_block_to_abort_hash = self
            .blocks
            .last_block_hash
            .ok_or(Error::UnsupportedAction { msg: "No blocks to abort".into() })?;
        let mut reached_starting_block = false;
        let mut aborted: Vec<Felt> = Vec::new();

        let mut rpc_contract_classes = self.rpc_contract_classes.write();

        // Abort blocks from latest to starting (iterating backwards) and revert transactions.
        while !reached_starting_block {
            reached_starting_block = next_block_to_abort_hash == starting_block_hash;

            let aborted_block = self.blocks.remove(&next_block_to_abort_hash).ok_or(
                Error::UnexpectedInternalError {
                    msg: format!("Cannot find block for abortion: {next_block_to_abort_hash:#x}"),
                },
            )?;

            // Revert transactions
            for tx_hash in aborted_block.get_transactions() {
                self.transactions.remove(tx_hash).ok_or(Error::UnexpectedInternalError {
                    msg: format!("No tx of abortable block: {tx_hash:#x}"),
                })?;
            }

            rpc_contract_classes.remove_classes_at(aborted_block.block_number().0);
            aborted.push(aborted_block.block_hash());

            // Update next block hash to abort
            next_block_to_abort_hash = aborted_block.parent_hash();
        }
        let last_unaborted_block_hash = next_block_to_abort_hash;

        // Update last_block_hash based on last reached block and revert state only if
        // starting block is reached in while loop.
        if reached_starting_block {
            self.blocks.last_block_hash = Some(last_unaborted_block_hash);

            let reverted_state = self.blocks.hash_to_state.get(&last_unaborted_block_hash).ok_or(
                Error::NoStateAtBlock { block_id: BlockId::Hash(last_unaborted_block_hash) },
            )?;

            // In the abort block scenario, we need to revert state and pre_confirmed_state to be
            // able to use the calls properly.
            self.latest_state = reverted_state.clone_historic();
            self.pre_confirmed_state = reverted_state.clone_historic();
        }

        self.pre_confirmed_state_diff = StateDiff::default();
        rpc_contract_classes.empty_staging();
        drop(rpc_contract_classes); // to later allow set_block_number
        self.blocks.aborted_blocks.extend_from_slice(&aborted);

        // Pre-confirmed block is empty, but block number needs to be modified.
        let old_pre_confirmed_block_number = self.blocks.pre_confirmed_block.block_number().0;
        let new_pre_confirmed_block_number = old_pre_confirmed_block_number - aborted.len() as u64;

        self.set_block_number(new_pre_confirmed_block_number);

        Ok(aborted)
    }

    pub fn last_aborted_block_hash(&self) -> Option<&BlockHash> {
        self.blocks.aborted_blocks.last()
    }

    fn validate_acceptability_on_l1(&self, block_status: BlockStatus) -> DevnetResult<()> {
        let err_msg = match block_status {
            BlockStatus::AcceptedOnL2 => return Ok(()),
            BlockStatus::PreConfirmed => "Pre-confirmed block cannot be accepted on L1",
            BlockStatus::AcceptedOnL1 => "Block already accepted on L1",
            BlockStatus::Rejected => {
                "Rejected blocks should no longer exist; if you see this message, please report a bug at https://github.com/0xSpaceShard/starknet-devnet/issues/new"
            }
        };

        Err(Error::UnsupportedAction { msg: err_msg.into() })
    }

    pub fn accept_on_l1(&mut self, block_id: BlockId) -> DevnetResult<Vec<BlockHash>> {
        let block = self.get_block(&block_id)?;
        // Only the starting block is validated; all ancestors are guaranteed to be ACCEPTED_ON_L2
        self.validate_acceptability_on_l1(block.status)?;

        let mut acceptable_block =
            self.blocks.hash_to_block.get_mut(&block.block_hash()).ok_or(Error::NoBlock)?;
        let mut accepted = vec![];
        while acceptable_block.status != BlockStatus::AcceptedOnL1 {
            // Accept block on L1
            acceptable_block.status = BlockStatus::AcceptedOnL1;
            accepted.push(acceptable_block.block_hash());

            // Mark the transactions from the block as accepted on L1
            for tx_hash in acceptable_block.get_transactions() {
                let tx = self.transactions.get_by_hash_mut(tx_hash).ok_or(Error::NoTransaction)?;
                tx.finality_status = TransactionFinalityStatus::AcceptedOnL1;
            }

            // Get next block for accepting: parent
            let parent_block_hash = acceptable_block.parent_hash();
            match self.blocks.hash_to_block.get_mut(&parent_block_hash) {
                Some(parent_block) => acceptable_block = parent_block,
                None => break, // Means genesis block is encountered
            }
        }

        Ok(accepted)
    }

    pub fn get_block_txs_count(&self, block_id: &BlockId) -> DevnetResult<u64> {
        let block = self.get_block(block_id)?;
        Ok(block.get_transactions().len() as u64)
    }

    pub fn contract_nonce_at_block(
        &mut self,
        block_id: &BlockId,
        contract_address: ContractAddress,
    ) -> DevnetResult<Felt> {
        let state = self.get_mut_state_at(block_id)?;
        state.assert_contract_deployed(contract_address)?;
        let nonce = state.get_nonce_at(contract_address.into())?;
        Ok(nonce.0)
    }

    pub fn contract_storage_at_block(
        &mut self,
        block_id: &BlockId,
        contract_address: ContractAddress,
        storage_key: PatriciaKey,
    ) -> DevnetResult<Felt> {
        let state = self.get_mut_state_at(block_id)?;
        state.assert_contract_deployed(contract_address)?;
        Ok(state.get_storage_at(contract_address.into(), storage_key.into())?)
    }

    pub fn get_block(&self, block_id: &BlockId) -> DevnetResult<&StarknetBlock> {
        self.blocks.get_by_block_id(block_id).ok_or(Error::NoBlock)
    }

    pub fn get_block_with_transactions(&self, block_id: &BlockId) -> DevnetResult<BlockResult> {
        let block = self.get_block(block_id)?;
        let transactions = block
            .get_transactions()
            .iter()
            .map(|transaction_hash| {
                self.transactions
                    .get_by_hash(*transaction_hash)
                    .ok_or(Error::NoTransaction)
                    .map(|transaction| transaction.inner.clone())
            })
            .collect::<DevnetResult<Vec<TransactionWithHash>>>()?;

        if block.status() == &BlockStatus::PreConfirmed {
            Ok(BlockResult::PreConfirmedBlock(PreConfirmedBlock {
                header: PreConfirmedBlockHeader::from(block),
                transactions: Transactions::Full(transactions),
            }))
        } else {
            Ok(BlockResult::Block(Block {
                status: *block.status(),
                header: BlockHeader::from(block),
                transactions: Transactions::Full(transactions),
            }))
        }
    }

    pub fn get_block_with_receipts(&self, block_id: &BlockId) -> DevnetResult<BlockResult> {
        let block = self.get_block(block_id)?;
        let mut transaction_receipts: Vec<TransactionWithReceipt> = vec![];

        for transaction_hash in block.get_transactions() {
            let sn_transaction =
                self.transactions.get_by_hash(*transaction_hash).ok_or(Error::NoTransaction)?;

            let transaction = sn_transaction.inner.clone();
            let receipt = sn_transaction.get_receipt()?;

            transaction_receipts
                .push(TransactionWithReceipt { receipt, transaction: transaction.transaction });
        }

        if block.status() == &BlockStatus::PreConfirmed {
            Ok(BlockResult::PreConfirmedBlock(PreConfirmedBlock {
                header: PreConfirmedBlockHeader::from(block),
                transactions: Transactions::FullWithReceipts(transaction_receipts),
            }))
        } else {
            Ok(BlockResult::Block(Block {
                status: *block.status(),
                header: BlockHeader::from(block),
                transactions: Transactions::FullWithReceipts(transaction_receipts),
            }))
        }
    }

    pub fn get_transaction_by_block_id_and_index(
        &self,
        block_id: &BlockId,
        index: u64,
    ) -> DevnetResult<&TransactionWithHash> {
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
            .get_by_block_id(&BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
            .ok_or(crate::error::Error::NoBlock)?;

        Ok(block.clone())
    }

    pub fn get_transaction_by_hash(
        &self,
        transaction_hash: Felt,
    ) -> DevnetResult<&TransactionWithHash> {
        self.transactions
            .get_by_hash(transaction_hash)
            .map(|starknet_transaction| &starknet_transaction.inner)
            .ok_or(Error::NoTransaction)
    }

    pub fn get_unlimited_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<ContractAddress>,
        keys: Option<Vec<Vec<Felt>>>,
    ) -> DevnetResult<Vec<EmittedEvent>> {
        events::get_events(self, from_block, to_block, address, keys, 0, None)
            .map(|(emitted_events, _)| emitted_events)
    }

    pub fn get_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<ContractAddress>,
        keys: Option<Vec<Vec<Felt>>>,
        skip: u64,
        limit: Option<u64>,
    ) -> DevnetResult<(Vec<EmittedEvent>, bool)> {
        events::get_events(self, from_block, to_block, address, keys, skip, limit)
    }

    pub fn get_transaction_receipt_by_hash(
        &self,
        transaction_hash: &TransactionHash,
    ) -> DevnetResult<TransactionReceipt> {
        let transaction_to_map =
            self.transactions.get(transaction_hash).ok_or(Error::NoTransaction)?;

        transaction_to_map.get_receipt()
    }

    pub fn get_transaction_trace_by_hash(
        &self,
        transaction_hash: TransactionHash,
    ) -> DevnetResult<TransactionTrace> {
        let tx = self.transactions.get(&transaction_hash).ok_or(Error::NoTransaction)?;
        tx.get_trace().ok_or(Error::NoTransactionTrace)
    }

    pub fn get_transaction_traces_from_block(
        &self,
        block_id: &BlockId,
    ) -> DevnetResult<Vec<BlockTransactionTrace>> {
        let transactions = match self.get_block_with_transactions(block_id)? {
            BlockResult::Block(b) => b.transactions,
            BlockResult::PreConfirmedBlock(b) => b.transactions,
        };

        let mut traces = Vec::new();
        if let Transactions::Full(txs) = transactions {
            for tx in txs {
                let tx_hash = *tx.get_transaction_hash();
                let trace = self.get_transaction_trace_by_hash(tx_hash)?;
                let block_trace =
                    BlockTransactionTrace { transaction_hash: tx_hash, trace_root: trace };

                traces.push(block_trace);
            }
        }

        Ok(traces)
    }

    pub fn get_transaction_execution_and_finality_status(
        &self,
        transaction_hash: TransactionHash,
    ) -> DevnetResult<TransactionStatus> {
        let transaction = self.transactions.get(&transaction_hash).ok_or(Error::NoTransaction)?;
        Ok(transaction.get_status())
    }

    pub fn simulate_transactions(
        &mut self,
        block_id: &BlockId,
        transactions: &[BroadcastedTransaction],
        simulation_flags: Vec<SimulationFlag>,
    ) -> DevnetResult<Vec<SimulatedTransaction>> {
        let chain_id = self.chain_id().to_felt();
        let block_context = self.block_context.clone();

        let mut skip_validate = false;
        let mut skip_fee_charge = false;
        for flag in simulation_flags.iter() {
            match flag {
                SimulationFlag::SkipValidate => {
                    skip_validate = true;
                }
                SimulationFlag::SkipFeeCharge => skip_fee_charge = true,
            }
        }
        let using_pre_confirmed_block = self.config.uses_pre_confirmed_block();

        let mut transactions_traces: Vec<TransactionTrace> = vec![];
        let cheats = self.cheats.clone();
        let state = self.get_mut_state_at(block_id)?;

        let executable_txs = {
            transactions
                .iter()
                .enumerate()
                .map(|(tx_idx, txn)| {
                    // According to this conversation https://spaceshard.slack.com/archives/C03HL8DH52N/p1710683496750409, simulating a transaction will:
                    // fail if the fee provided is 0
                    // succeed if the fee provided is 0 and SKIP_FEE_CHARGE is set
                    // succeed if the fee provided is > 0
                    if !skip_fee_charge && !txn.are_gas_bounds_valid()  {
                        return Err(Error::ContractExecutionErrorInSimulation {
                            failure_index: tx_idx,
                            execution_error: ContractExecutionError::from(TransactionValidationError::InsufficientResourcesForValidate
                                .to_string()),
                        });
                    }

                    let skip_validate_due_to_impersonation =
                        Starknet::should_transaction_skip_validation_if_sender_is_impersonated(
                            state, &cheats, txn,
                        )?;

                    Ok((
                        txn.to_blockifier_account_transaction(&chain_id, ExecutionFlags {
                            only_query: true,
                            charge_fee: !skip_fee_charge,
                            validate: !(skip_validate || skip_validate_due_to_impersonation),
                            strict_nonce_check: txn.requires_strict_nonce_check(using_pre_confirmed_block),
                        })?,
                        txn.get_type(),
                        txn.gas_vector_computation_mode(),
                    ))
                })
                .collect::<DevnetResult<Vec<(AccountTransaction, TransactionType, GasVectorComputationMode)>>>()?
        };

        let transactional_rpc_contract_classes =
            Arc::new(RwLock::new(state.clone_rpc_contract_classes()));
        let mut transactional_state =
            CachedState::new(CachedState::create_transactional(&mut state.state));

        for (tx_idx, (blockifier_transaction, transaction_type, gas_vector_computation_mode)) in
            executable_txs.into_iter().enumerate()
        {
            let tx_execution_info = blockifier_transaction
                .execute(&mut transactional_state, &block_context)
                .map_err(|err| Error::ContractExecutionErrorInSimulation {
                    failure_index: tx_idx,
                    execution_error: ContractExecutionError::from(err),
                })?;

            let block_number = block_context.block_info().block_number.0;
            let new_classes = transactional_rpc_contract_classes.write().commit(block_number);
            let state_diff: ThinStateDiff =
                StateDiff::generate(&mut transactional_state, new_classes)?.into();
            let trace = create_trace(
                &mut transactional_state,
                transaction_type,
                &tx_execution_info,
                state_diff,
                block_context.versioned_constants(),
                &gas_vector_computation_mode,
            )?;
            transactions_traces.push(trace);
        }

        let estimated = estimations::estimate_fee(
            self,
            block_id,
            transactions,
            Some(!skip_fee_charge),
            Some(!skip_validate),
            false,
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

    /// create new block from pre_confirmed one
    pub fn create_block(&mut self) -> DevnetResult<(), Error> {
        self.generate_new_block_and_state()?;
        Ok(())
    }

    // Set time and optionally create a new block
    pub fn set_time(&mut self, timestamp: u64, create_block: bool) -> DevnetResult<(), Error> {
        self.set_block_timestamp_shift(
            timestamp as i64 - Starknet::get_unix_timestamp_as_seconds() as i64,
        );

        self.set_next_block_timestamp(timestamp);
        if create_block {
            self.create_block()?;
        }

        Ok(())
    }

    // Set timestamp shift and create empty block
    pub fn increase_time(&mut self, time_shift: u64) -> DevnetResult<(), Error> {
        self.set_block_timestamp_shift(
            self.pre_confirmed_block_timestamp_shift + time_shift as i64,
        );
        self.create_block()
    }

    // Set timestamp shift for next blocks
    pub fn set_block_timestamp_shift(&mut self, timestamp: i64) {
        self.pre_confirmed_block_timestamp_shift = timestamp;
    }

    // Set next block timestamp
    pub fn set_next_block_timestamp(&mut self, timestamp: u64) {
        self.next_block_timestamp = Some(timestamp);
    }

    #[allow(clippy::expect_used)]
    pub fn get_unix_timestamp_as_seconds() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("should get current UNIX timestamp")
            .as_secs()
    }

    /// Impersonates account, allowing to send transactions on behalf of the account, without its
    /// private key
    ///
    /// # Arguments
    /// * `account` - Account to impersonate
    pub fn impersonate_account(&mut self, account: ContractAddress) -> DevnetResult<(), Error> {
        if self.config.fork_config.url.is_none() {
            return Err(Error::UnsupportedAction {
                msg: "Account impersonation is supported when forking mode is enabled.".to_string(),
            });
        }
        if self.pre_confirmed_state.is_contract_deployed_locally(account)? {
            return Err(Error::UnsupportedAction {
                msg: "Account is in local state, cannot be impersonated".to_string(),
            });
        }
        self.cheats.impersonate_account(account);
        Ok(())
    }

    /// Stops impersonating account.
    /// After this call, the account, previously impersonated can't be used to send transactions
    /// without its private key
    ///
    /// # Arguments
    /// * `account` - Account to stop impersonating
    pub fn stop_impersonating_account(&mut self, account: &ContractAddress) {
        self.cheats.stop_impersonating_account(account);
    }

    /// Turn on/off auto impersonation of accounts that are not part of the state
    ///
    /// # Arguments
    /// * `auto_impersonation` - If true, auto impersonate every account that is not part of the
    ///   state, otherwise dont auto impersonate
    pub fn set_auto_impersonate_account(
        &mut self,
        auto_impersonation: bool,
    ) -> DevnetResult<(), Error> {
        if self.config.fork_config.url.is_none() {
            return Err(Error::UnsupportedAction {
                msg: "Account impersonation is supported when forking mode is enabled.".to_string(),
            });
        }
        self.cheats.set_auto_impersonate(auto_impersonation);

        Ok(())
    }

    /// Returns true if the account is not part of the state and is impersonated
    ///
    /// # Arguments
    /// * `account` - Account to check
    fn is_account_impersonated(
        state: &mut StarknetState,
        cheats: &Cheats,
        account: &ContractAddress,
    ) -> DevnetResult<bool> {
        let is_contract_already_in_state = state.is_contract_deployed_locally(*account)?;
        if is_contract_already_in_state {
            return Ok(false);
        }

        Ok(cheats.is_impersonated(account))
    }

    /// Returns true if the transaction should skip validation if the sender is impersonated
    ///
    /// # Arguments
    /// * `transaction` - Transaction to check
    fn should_transaction_skip_validation_if_sender_is_impersonated(
        state: &mut StarknetState,
        cheats: &Cheats,
        transaction: &BroadcastedTransaction,
    ) -> DevnetResult<bool> {
        let sender_address = match transaction {
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(v3)) => {
                Some(&v3.sender_address)
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(v3)) => {
                Some(&v3.sender_address)
            }
            BroadcastedTransaction::DeployAccount(_) => None,
        };

        if let Some(sender_address) = sender_address {
            Starknet::is_account_impersonated(state, cheats, sender_address)
        } else {
            Ok(false)
        }
    }

    pub fn get_messages_status(
        &self,
        l1_tx_hash: Hash256,
    ) -> Option<Vec<L1HandlerTransactionStatus>> {
        match self.messaging.l1_to_l2_tx_hashes.get(&H256(*l1_tx_hash.as_bytes())) {
            Some(l2_tx_hashes) => {
                let mut statuses = vec![];
                for l2_tx_hash in l2_tx_hashes {
                    match self.transactions.get(l2_tx_hash) {
                        Some(l2_tx) => statuses.push(L1HandlerTransactionStatus {
                            transaction_hash: *l2_tx_hash,
                            finality_status: l2_tx.finality_status,
                            execution_status: l2_tx.execution_result.status(),
                            failure_reason: maybe_extract_failure_reason(&l2_tx.execution_info),
                        }),
                        // should never happen due to handling in add_l1_handler_transaction
                        None => return None,
                    }
                }
                Some(statuses)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use blockifier::state::state_api::{State, StateReader};
    use nonzero_ext::nonzero;
    use starknet_api::block::{BlockHash, BlockNumber, BlockTimestamp, FeeType};
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::rpc::block::BlockStatus;
    use starknet_types::rpc::state::Balance;
    use starknet_types::rpc::transaction_receipt::TransactionReceipt;
    use starknet_types::traits::HashProducer;

    use super::Starknet;
    use super::starknet_config::BlockGenerationOn;
    use crate::account::{Account, FeeToken};
    use crate::blocks::StarknetBlock;
    use crate::constants::{
        ARGENT_CONTRACT_CLASS_HASH, ARGENT_MULTISIG_CONTRACT_CLASS_HASH,
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_INITIAL_BALANCE,
        DEVNET_DEFAULT_STARTING_BLOCK_NUMBER, ETH_ERC20_CONTRACT_ADDRESS,
        STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::{DevnetResult, Error};
    use crate::starknet::starknet_config::{StarknetConfig, StateArchiveCapacity};
    use crate::traits::{Accounted, Deployed, HashIdentified};
    use crate::utils::test_utils::{
        broadcasted_declare_tx_v3_of_dummy_class, cairo_0_account_without_validations,
        dummy_contract_address, dummy_declare_tx_v3_with_hash, dummy_felt, dummy_key_pair,
        resource_bounds_with_price_1,
    };

    /// Initializes starknet with 1 account that doesn't perform actual tx signature validation.
    /// Allows specifying the state archive capacity.
    pub(crate) fn setup_starknet_with_no_signature_check_account_and_state_capacity(
        acc_balance: u128,
        state_archive: StateArchiveCapacity,
    ) -> (Starknet, Account) {
        let mut starknet = Starknet::new(&StarknetConfig {
            gas_price_wei: nonzero!(1u128),
            gas_price_fri: nonzero!(1u128),
            data_gas_price_wei: nonzero!(1u128),
            data_gas_price_fri: nonzero!(1u128),
            l2_gas_price_wei: nonzero!(1u128),
            l2_gas_price_fri: nonzero!(1u128),
            state_archive,
            ..Default::default()
        })
        .unwrap();

        let account_class = cairo_0_account_without_validations();
        let acc = Account::new(
            Balance::from(acc_balance),
            dummy_key_pair(),
            account_class.generate_hash().unwrap(),
            "Custom",
            account_class.into(),
            starknet.block_context.chain_info().fee_token_addresses.eth_fee_token_address.into(),
            starknet.block_context.chain_info().fee_token_addresses.strk_fee_token_address.into(),
        )
        .unwrap();
        acc.deploy(&mut starknet.pre_confirmed_state).unwrap();

        starknet.commit_diff().unwrap();
        starknet.generate_new_block_and_state().unwrap();
        starknet.restart_pre_confirmed_block().unwrap();

        (starknet, acc)
    }

    /// Initializes starknet with 1 account that doesn't perform actual tx signature validation.
    pub(crate) fn setup_starknet_with_no_signature_check_account(
        acc_balance: u128,
    ) -> (Starknet, Account) {
        setup_starknet_with_no_signature_check_account_and_state_capacity(
            acc_balance,
            StateArchiveCapacity::None,
        )
    }

    #[test]
    fn correct_initial_state_with_test_config() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        let predeployed_accounts = starknet.predeployed_accounts.get_accounts();
        let expected_balance = config.predeployed_accounts_initial_balance;

        for account in predeployed_accounts {
            let account_balance =
                account.get_balance(&mut starknet.pre_confirmed_state, FeeToken::ETH).unwrap();
            assert_eq!(expected_balance, account_balance);

            let account_balance =
                account.get_balance(&mut starknet.pre_confirmed_state, FeeToken::STRK).unwrap();
            assert_eq!(expected_balance, account_balance);
        }
    }

    #[test]
    fn correct_block_context_creation() {
        let fee_token_address =
            ContractAddress::new(felt_from_prefixed_hex("0xAA").unwrap()).unwrap();
        let block_ctx = Starknet::init_block_context(
            nonzero!(10u128),
            nonzero!(10u128),
            nonzero!(10u128),
            nonzero!(10u128),
            nonzero!(10u128),
            nonzero!(10u128),
            felt_from_prefixed_hex("0xAA").unwrap(),
            STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );
        assert_eq!(block_ctx.block_info().block_number, BlockNumber(0));
        assert_eq!(block_ctx.block_info().block_timestamp, BlockTimestamp(0));
        assert_eq!(block_ctx.block_info().gas_prices.l1_gas_price(&FeeType::Eth).get().0, 10);
        assert_eq!(
            ContractAddress::from(block_ctx.chain_info().fee_token_addresses.eth_fee_token_address),
            fee_token_address
        );
    }

    #[test]
    fn pre_confirmed_block_is_correct() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        let initial_block_number = starknet.block_context.block_info().block_number;
        starknet.generate_pre_confirmed_block().unwrap();

        assert_eq!(
            starknet.pre_confirmed_block().header.block_header_without_hash.block_number,
            initial_block_number.next().unwrap()
        );
    }

    #[test]
    fn correct_new_block_creation() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let tx = dummy_declare_tx_v3_with_hash();

        // add transaction hash to pre_confirmed block
        starknet.blocks.pre_confirmed_block.add_transaction(*tx.get_transaction_hash());

        // pre_confirmed block has some transactions
        assert!(!starknet.pre_confirmed_block().get_transactions().is_empty());
        // blocks collection should not be empty
        assert_eq!(starknet.blocks.hash_to_block.len(), 1);

        starknet.generate_new_block_and_state().unwrap();
        // blocks collection should not be empty
        assert_eq!(starknet.blocks.hash_to_block.len(), 2);

        // get latest block and check that the transactions in the block are correct
        let added_block =
            starknet.blocks.get_by_hash(starknet.blocks.last_block_hash.unwrap()).unwrap();

        assert!(added_block.get_transactions().len() == 1);
        assert_eq!(*added_block.get_transactions().first().unwrap(), *tx.get_transaction_hash());
    }

    #[test]
    fn successful_emptying_of_pre_confirmed_block() {
        let config = StarknetConfig { start_time: Some(0), ..Default::default() };
        let mut starknet = Starknet::new(&config).unwrap();

        let initial_block_number = starknet.block_context.block_info().block_number;
        let initial_gas_price_wei =
            starknet.block_context.block_info().gas_prices.l1_gas_price(&FeeType::Eth);
        let initial_gas_price_fri =
            starknet.block_context.block_info().gas_prices.l1_gas_price(&FeeType::Strk);
        let initial_data_gas_price_wei =
            starknet.block_context.block_info().gas_prices.l1_gas_price(&FeeType::Eth);
        let initial_data_gas_price_fri =
            starknet.block_context.block_info().gas_prices.l1_data_gas_price(&FeeType::Strk);
        let initial_block_timestamp = starknet.block_context.block_info().block_timestamp;
        let initial_sequencer = starknet.block_context.block_info().sequencer_address;

        // create pre_confirmed block with some information in it
        let mut pre_confirmed_block = StarknetBlock::create_pre_confirmed_block();
        pre_confirmed_block.add_transaction(dummy_felt());
        pre_confirmed_block.status = BlockStatus::AcceptedOnL2;

        // assign the pre_confirmed block
        starknet.blocks.pre_confirmed_block = pre_confirmed_block.clone();
        assert!(*starknet.pre_confirmed_block() == pre_confirmed_block);

        // empty the pre_confirmed to block and check if it is in starting state
        starknet.restart_pre_confirmed_block().unwrap();

        assert!(*starknet.pre_confirmed_block() != pre_confirmed_block);
        assert_eq!(starknet.pre_confirmed_block().status, BlockStatus::PreConfirmed);
        assert!(starknet.pre_confirmed_block().get_transactions().is_empty());

        let header = &starknet.pre_confirmed_block().header.block_header_without_hash;
        assert_eq!(header.timestamp, initial_block_timestamp);
        assert_eq!(header.block_number, initial_block_number);
        assert_eq!(header.parent_hash, BlockHash::default());
        assert_eq!(header.l1_gas_price.price_in_wei, initial_gas_price_wei.get());
        assert_eq!(header.l1_gas_price.price_in_fri, initial_gas_price_fri.get());
        assert_eq!(header.l1_data_gas_price.price_in_wei, initial_data_gas_price_wei.get());
        assert_eq!(header.l1_data_gas_price.price_in_fri, initial_data_gas_price_fri.get());
        assert_eq!(header.l2_gas_price.price_in_fri, initial_gas_price_fri.get());
        assert_eq!(header.sequencer.0, initial_sequencer);
    }

    #[test]
    fn correct_block_context_update() {
        let mut block_ctx = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            ETH_ERC20_CONTRACT_ADDRESS,
            STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );
        let initial_block_number = block_ctx.block_info().block_number;
        Starknet::advance_block_context_block_number(&mut block_ctx);

        assert_eq!(block_ctx.block_info().block_number, initial_block_number.next().unwrap());
    }

    #[test]
    fn getting_state_of_latest_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.get_mut_state_at(&BlockId::Tag(BlockTag::Latest)).expect("Should be OK");
    }

    #[test]
    fn getting_state_of_pre_confirmed_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.get_mut_state_at(&BlockId::Tag(BlockTag::PreConfirmed)).expect("Should be OK");
    }

    #[test]
    fn getting_state_at_block_by_nonexistent_hash_with_full_state_archive() {
        let config =
            StarknetConfig { state_archive: StateArchiveCapacity::Full, ..Default::default() };
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block_and_state().unwrap();

        match starknet.get_mut_state_at(&BlockId::Hash(Felt::ZERO)) {
            Err(Error::NoBlock) => (),
            _ => panic!("Should fail with NoBlock"),
        }
    }

    #[test]
    fn getting_nonexistent_state_at_block_by_number_with_full_state_archive() {
        let config =
            StarknetConfig { state_archive: StateArchiveCapacity::Full, ..Default::default() };
        let mut starknet = Starknet::new(&config).unwrap();
        let genesis_block_hash = starknet.get_latest_block().unwrap();
        let block_hash = starknet.generate_new_block_and_state().unwrap();
        starknet.blocks.hash_to_state.remove(&block_hash);
        starknet.blocks.last_block_hash = Some(genesis_block_hash.block_hash());

        match starknet.get_mut_state_at(&BlockId::Number(1)) {
            Err(Error::NoStateAtBlock { block_id: _ }) => (),
            _ => panic!("Should fail with NoStateAtBlock"),
        }
    }

    #[test]
    fn getting_state_at_without_state_archive() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block_and_state().unwrap();

        match starknet.get_mut_state_at(&BlockId::Number(0)) {
            Err(Error::NoStateAtBlock { .. }) => (),
            _ => panic!("Should fail with NoStateAtBlock."),
        }
    }

    #[test]
    fn assert_expected_predeclared_account_classes() {
        let config = StarknetConfig { predeclare_argent: true, ..Default::default() };
        let starknet = Starknet::new(&config).unwrap();
        for class_hash in [
            ARGENT_CONTRACT_CLASS_HASH,
            ARGENT_MULTISIG_CONTRACT_CLASS_HASH,
            Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
            Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH),
        ] {
            let contract = starknet.get_class(&BlockId::Tag(BlockTag::Latest), class_hash).unwrap();
            assert_eq!(contract.generate_hash().unwrap(), class_hash);
        }
    }

    #[test]
    fn calling_method_of_undeployed_contract() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let undeployed_address = Felt::from_hex_unchecked("0x1234");
        let entry_point_selector = get_selector_from_name("balanceOf").unwrap();

        match starknet.call(
            &BlockId::Tag(BlockTag::Latest),
            undeployed_address,
            entry_point_selector,
            vec![],
        ) {
            Err(Error::ContractNotFound) => (),
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    #[test]
    fn calling_nonexistent_contract_method() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let predeployed_account = &starknet.predeployed_accounts.get_accounts()[0];
        let entry_point_selector = get_selector_from_name("nonExistentMethod").unwrap();

        match starknet.call(
            &BlockId::Tag(BlockTag::Latest),
            ETH_ERC20_CONTRACT_ADDRESS,
            entry_point_selector,
            vec![Felt::from(predeployed_account.account_address)],
        ) {
            Err(Error::EntrypointNotFound) => (),
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    /// utility method for happy path balance retrieval
    fn get_balance_at(
        starknet: &mut Starknet,
        contract_address: ContractAddress,
    ) -> DevnetResult<Vec<Felt>> {
        let entry_point_selector = get_selector_from_name("balanceOf").unwrap();
        starknet.call(
            &BlockId::Tag(BlockTag::Latest),
            ETH_ERC20_CONTRACT_ADDRESS,
            entry_point_selector,
            vec![Felt::from(contract_address)],
        )
    }

    #[test]
    fn getting_balance_of_predeployed_contract() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let predeployed_account = &starknet.predeployed_accounts.get_accounts()[0].clone();
        let result = get_balance_at(&mut starknet, predeployed_account.account_address).unwrap();

        let balance_uint256 = vec![Felt::from(DEVNET_DEFAULT_INITIAL_BALANCE), Felt::ZERO];
        assert_eq!(result, balance_uint256);
    }

    #[test]
    fn getting_balance_of_undeployed_contract() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        let undeployed_address = ContractAddress::new(Felt::from_hex_unchecked("0x1234")).unwrap();
        let result = get_balance_at(&mut starknet, undeployed_address).unwrap();

        let expected_balance_uint256 = vec![Felt::ZERO, Felt::ZERO];
        assert_eq!(result, expected_balance_uint256);
    }

    #[test]
    fn correct_latest_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        // last added block number -> 0
        let added_block =
            starknet.blocks.get_by_hash(starknet.blocks.last_block_hash.unwrap()).unwrap();
        // number of the accepted block -> 1
        let block_number = starknet.get_latest_block().unwrap().block_number();

        assert_eq!(block_number.0, added_block.header.block_header_without_hash.block_number.0);

        starknet.generate_new_block_and_state().unwrap();

        let added_block2 =
            starknet.blocks.get_by_hash(starknet.blocks.last_block_hash.unwrap()).unwrap();
        let block_number2 = starknet.get_latest_block().unwrap().block_number();

        assert_eq!(block_number2.0, added_block2.header.block_header_without_hash.block_number.0);
    }

    #[test]
    fn gets_block_txs_count() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block_and_state().unwrap();

        let num_no_transactions = starknet.get_block_txs_count(&BlockId::Number(1));

        assert_eq!(num_no_transactions.unwrap(), 0);

        let tx = dummy_declare_tx_v3_with_hash();

        // add transaction hash to pre_confirmed block
        starknet.blocks.pre_confirmed_block.add_transaction(*tx.get_transaction_hash());

        starknet.generate_new_block_and_state().unwrap();

        let num_one_transaction = starknet.get_block_txs_count(&BlockId::Number(2));

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
        let mut starknet = Starknet::new(&StarknetConfig {
            state_archive: StateArchiveCapacity::Full,
            ..Default::default()
        })
        .expect("Could not start Devnet");

        // generate initial block with empty state
        starknet.generate_new_block_and_state().unwrap();

        // **generate second block**
        // add data to state
        starknet
            .pre_confirmed_state
            .state
            .increment_nonce(dummy_contract_address().into())
            .unwrap();

        // generate new block and save the state
        starknet.commit_diff().unwrap();
        let second_block = starknet.generate_new_block_and_state().unwrap();

        // **generate third block**
        // add data to state
        starknet
            .pre_confirmed_state
            .state
            .increment_nonce(dummy_contract_address().into())
            .unwrap();

        // generate new block and save the state
        starknet.commit_diff().unwrap();
        let third_block = starknet.generate_new_block_and_state().unwrap();

        // check modified state at block 1 and 2 to contain the correct value for the nonce
        let second_block_address_nonce = starknet
            .blocks
            .hash_to_state
            .get_mut(&second_block)
            .unwrap()
            .get_nonce_at(dummy_contract_address().into())
            .unwrap();
        let second_block_expected_address_nonce = Felt::ONE;
        assert_eq!(second_block_expected_address_nonce, second_block_address_nonce.0);

        let third_block_address_nonce = starknet
            .blocks
            .hash_to_state
            .get_mut(&third_block)
            .unwrap()
            .get_nonce_at(dummy_contract_address().into())
            .unwrap();
        let third_block_expected_address_nonce = Felt::TWO;
        assert_eq!(third_block_expected_address_nonce, third_block_address_nonce.0);
    }

    #[test]
    fn gets_latest_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block_and_state().unwrap();
        starknet.generate_new_block_and_state().unwrap();
        starknet.generate_new_block_and_state().unwrap();

        let latest_block = starknet.get_latest_block();

        assert_eq!(latest_block.unwrap().block_number(), BlockNumber(3));
    }
    #[test]
    fn check_timestamp_of_newly_generated_block() {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block_and_state().unwrap();
        starknet
            .blocks
            .pre_confirmed_block
            .set_timestamp(BlockTimestamp(Starknet::get_unix_timestamp_as_seconds()));
        let pre_confirmed_block_timestamp =
            starknet.pre_confirmed_block().header.block_header_without_hash.timestamp;

        let sleep_duration_secs = 5;
        thread::sleep(Duration::from_secs(sleep_duration_secs));
        starknet.generate_new_block_and_state().unwrap();

        let block_timestamp =
            starknet.get_latest_block().unwrap().header.block_header_without_hash.timestamp;
        // check if the pre_confirmed_block_timestamp is less than the block_timestamp,
        // by number of sleep seconds because the timeline of events is this:
        // ----(pre_confirmed block timestamp)----(sleep)----(new block timestamp)
        assert!(pre_confirmed_block_timestamp.0 + sleep_duration_secs <= block_timestamp.0);
    }

    #[test]
    fn test_block_abortion_when_state_archive_capacity_not_full() {
        let mut starknet = Starknet::new(&StarknetConfig {
            state_archive: StateArchiveCapacity::None,
            ..Default::default()
        })
        .unwrap();

        let dummy_hash = felt_from_prefixed_hex("0x42").unwrap();
        match starknet.abort_blocks(BlockId::Hash(dummy_hash)) {
            Err(Error::UnsupportedAction { msg }) => {
                assert!(msg.contains("state-archive-capacity"))
            }
            unexpected => panic!("Got unexpected response: {unexpected:?}"),
        }
    }

    #[test]
    fn test_abortion_of_non_existent_block() {
        let mut starknet = Starknet::new(&StarknetConfig {
            state_archive: StateArchiveCapacity::Full,
            ..Default::default()
        })
        .unwrap();

        let dummy_hash = felt_from_prefixed_hex("0x42").unwrap();
        match starknet.abort_blocks(BlockId::Hash(dummy_hash)) {
            Err(Error::NoBlock) => (),
            unexpected => panic!("Got unexpected response: {unexpected:?}"),
        }
    }

    #[test]
    fn receipt_should_have_block_properties_after_tx_is_accepted() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e18 as u128);
        starknet.config.block_generation_on = BlockGenerationOn::Demand;

        let tx = broadcasted_declare_tx_v3_of_dummy_class(
            sender.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        let (tx_hash, _) = starknet.add_declare_transaction(tx.into()).unwrap();

        let receipt = starknet.get_transaction_receipt_by_hash(&tx_hash).unwrap();
        match receipt {
            TransactionReceipt::Common(receipt) => {
                assert!(receipt.block_hash.is_none());
                assert!(receipt.block_number.is_none());
            }
            other => panic!("Unexpected receipt response: {other:?}"),
        }

        // receipt should have block params after accepting the tx by triggering block creation
        starknet.generate_new_block_and_state().unwrap();
        let latest_block = starknet.get_block(&BlockId::Tag(BlockTag::Latest)).unwrap();

        let receipt = starknet.get_transaction_receipt_by_hash(&tx_hash).unwrap();
        match receipt {
            TransactionReceipt::Common(receipt) => {
                assert_eq!(receipt.block_hash, Some(latest_block.block_hash()));
                assert_eq!(receipt.block_number, Some(BlockNumber(2))); // setup created block 1
            }
            other => panic!("Unexpected receipt response: {other:?}"),
        }
    }
}
