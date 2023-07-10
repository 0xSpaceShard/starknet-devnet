use std::collections::HashMap;
use std::time::SystemTime;

use blocks::{StarknetBlock, StarknetBlocks};
use constants::ERC20_CONTRACT_ADDRESS;
use predeployed_accounts::PredeployedAccounts;
use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
use starknet_in_rust::definitions::block_context::{BlockContext, StarknetOsConfig};
use starknet_in_rust::definitions::constants::{
    DEFAULT_CAIRO_RESOURCE_FEE_WEIGHTS, DEFAULT_CONTRACT_STORAGE_COMMITMENT_TREE_HEIGHT,
    DEFAULT_GLOBAL_STATE_COMMITMENT_TREE_HEIGHT, DEFAULT_INVOKE_TX_MAX_N_STEPS,
    DEFAULT_VALIDATE_MAX_N_STEPS,
};
use starknet_in_rust::execution::TransactionExecutionInfo;
use starknet_in_rust::state::BlockInfo;
use starknet_in_rust::testing::TEST_SEQUENCER_ADDRESS;
use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_rs_core::types::TransactionStatus;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;
use state::StarknetState;
use tracing::error;
use traits::{AccountGenerator, Accounted, HashIdentifiedMut, StateChanger};
use transactions::{StarknetTransaction, StarknetTransactions, Transaction};

pub mod account;
mod blocks;
mod constants;
mod predeployed_accounts;
mod services;
mod state;
mod system_contract;
mod traits;
mod transactions;
mod utils;

#[derive(Debug)]
pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    pub predeployed_accounts_initial_balance: Felt,
    pub host: String,
    pub port: u16,
    pub timeout: u16,
    pub gas_price: u64,
    pub chain_id: StarknetChainId,
}

#[derive(Default)]
pub struct Starknet {
    state: StarknetState,
    predeployed_accounts: PredeployedAccounts,
    block_context: BlockContext,
    blocks: StarknetBlocks,
    transactions: StarknetTransactions,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_fee_contract = Starknet::create_erc20()?;
        let udc_contract = Starknet::create_udc20()?;

        erc20_fee_contract.deploy(&mut state)?;
        udc_contract.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance,
            erc20_fee_contract.get_address(),
        );
        let account_contract_class =
            utils::load_cairo_0_contract_class(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
        let class_hash = account_contract_class.generate_hash()?;

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            class_hash,
            account_contract_class,
        )?;
        for account in accounts {
            account.deploy(&mut state)?;
            account.set_initial_balance(&mut state)?;
        }

        // copy already modified state to cached state
        state.synchronize_states();

        let mut this = Self {
            state,
            predeployed_accounts,
            block_context: Self::get_block_context(0, ERC20_CONTRACT_ADDRESS, config.chain_id)?,
            blocks: StarknetBlocks::default(),
            transactions: StarknetTransactions::default(),
        };

        this.restart_pending_block()?;

        Ok(this)
    }

    // Update block context
    // Initialize values for new pending block
    pub(crate) fn generate_pending_block(&mut self) -> DevnetResult<()> {
        Self::update_block_context(&mut self.block_context);
        self.restart_pending_block()?;

        Ok(())
    }

    // Transfer data from pending block into new block and save it to blocks collection
    pub(crate) fn generate_new_block(&mut self) -> DevnetResult<()> {
        let mut new_block = self.pending_block().clone();

        // set new block header
        new_block.set_block_hash(new_block.generate_hash()?);
        new_block.status = BlockStatus::AcceptedOnL2;

        // update txs block hash block number for each transaction in the pending block
        new_block.get_transactions().iter().for_each(|t| {
            if let Some(tx_hash) = t.get_hash() {
                if let Some(tx) = self.transactions.get_by_hash_mut(&tx_hash) {
                    tx.block_hash = Some(new_block.header.block_hash.0.into());
                    tx.block_number = Some(new_block.header.block_number);
                    tx.status = TransactionStatus::AcceptedOnL2;
                } else {
                    error!("Transaction is not present in the transactions colletion");
                }
            } else {
                error!("Transaction has no generated hash");
            }
        });

        // insert pending block in the blocks collection
        self.blocks.insert(new_block);

        Ok(())
    }

    pub(crate) fn handle_successful_transaction(
        &mut self,
        transaction_hash: &TransactionHash,
        transaction: Transaction,
        tx_info: TransactionExecutionInfo,
    ) -> DevnetResult<()> {
        let transaction_to_add =
            StarknetTransaction::create_successful(transaction.clone(), tx_info);

        // add accepted transaction to pending block
        self.blocks.pending_block.add_transaction(transaction);

        // add transaction to transactions
        self.transactions.insert(transaction_hash, transaction_to_add);

        // create new block from pending one
        self.generate_new_block()?;
        // apply state changes from cached state
        self.state.apply_cached_state()?;
        // make cached state part of "persistent" state
        self.state.synchronize_states();
        // clear pending block information
        self.generate_pending_block()?;

        Ok(())
    }

    fn get_block_context(gas_price: u64, fee_token_address: &str, chain_id: StarknetChainId) -> DevnetResult<BlockContext> {
        let starknet_os_config = StarknetOsConfig::new(
            chain_id,
            starknet_in_rust::utils::Address(
                Felt::from_prefixed_hex_str(fee_token_address)?.into(),
            ),
            gas_price as u128,
        );

        let mut block_info = BlockInfo::empty(TEST_SEQUENCER_ADDRESS.clone());
        block_info.gas_price = gas_price;

        let block_context = BlockContext::new(
            starknet_os_config,
            DEFAULT_CONTRACT_STORAGE_COMMITMENT_TREE_HEIGHT,
            DEFAULT_GLOBAL_STATE_COMMITMENT_TREE_HEIGHT,
            DEFAULT_CAIRO_RESOURCE_FEE_WEIGHTS.clone(),
            DEFAULT_INVOKE_TX_MAX_N_STEPS,
            DEFAULT_VALIDATE_MAX_N_STEPS,
            block_info,
            HashMap::default(),
            true,
        );

        Ok(block_context)
    }

    /// Should update block context with new block timestamp
    /// and pointer to the next block number
    fn update_block_context(block_context: &mut BlockContext) {
        let current_timestamp_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("should get current UNIX timestamp")
            .as_secs();

        block_context.block_info_mut().block_number = block_context.block_info().block_number + 1;
        block_context.block_info_mut().block_timestamp = current_timestamp_secs;
    }

    fn pending_block(&self) -> &StarknetBlock {
        &self.blocks.pending_block
    }

    /// Restarts pending block with information from block_context
    fn restart_pending_block(&mut self) -> DevnetResult<()> {
        let mut block = StarknetBlock::create_pending_block();

        block.header.block_number = BlockNumber(self.block_context.block_info().block_number);
        block.header.gas_price = GasPrice(self.block_context.block_info().gas_price.into());
        block.header.sequencer =
            ContractAddress::try_from(self.block_context.block_info().sequencer_address.clone())?
                .try_into()?;
        block.header.timestamp = BlockTimestamp(self.block_context.block_info().block_timestamp);

        self.blocks.pending_block = block;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{BlockHash, BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;
    use starknet_in_rust::definitions::block_context::StarknetChainId;

    use crate::blocks::StarknetBlock;
    use crate::traits::Accounted;
    use crate::utils::test_utils::dummy_declare_transaction_v1;
    use crate::{Starknet, StarknetConfig};

    pub(crate) fn starknet_config_for_test() -> StarknetConfig {
        StarknetConfig {
            seed: 123,
            total_accounts: 3,
            predeployed_accounts_initial_balance: 100.into(),
        }
    }

    #[test]
    fn correct_initial_state_with_test_config() {
        let config = starknet_config_for_test();
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
        let block_ctx = Starknet::get_block_context(10, "0xAA", StarknetChainId::TestNet).unwrap();
        assert!(block_ctx.block_info().block_number == 0);
        assert!(block_ctx.block_info().block_timestamp == 0);
        assert_eq!(block_ctx.block_info().gas_price, 10);
        assert_eq!(
            block_ctx.starknet_os_config().fee_token_address().clone(),
            fee_token_address.try_into().unwrap()
        );
    }

    #[test]
    fn pending_block_is_correct() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();
        let initial_block_number = starknet.block_context.block_info().block_number;
        starknet.generate_pending_block().unwrap();

        assert_eq!(
            starknet.pending_block().header.block_number,
            BlockNumber(initial_block_number + 1)
        );
    }

    #[test]
    fn correct_new_block_creation() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();

        let mut tx = dummy_declare_transaction_v1();
        let tx_hash = tx.generate_hash().unwrap();
        tx.transaction_hash = Some(tx_hash);

        // add transaction to pending block
        starknet
            .blocks
            .pending_block
            .add_transaction(crate::transactions::Transaction::Declare(tx));

        // pending block has some transactions
        assert!(!starknet.pending_block().get_transactions().is_empty());
        // blocks collection is empty
        assert!(starknet.blocks.num_to_block.is_empty());

        starknet.generate_new_block().unwrap();
        // blocks collection should not be empty
        assert!(!starknet.blocks.num_to_block.is_empty());

        // get block by number and check that the transactions in the block are correct
        let added_block = starknet.blocks.num_to_block.get(&BlockNumber(0)).unwrap();

        assert!(added_block.get_transactions().len() == 1);
        assert_eq!(added_block.get_transactions().first().unwrap().get_hash().unwrap(), tx_hash);
    }

    #[test]
    fn successful_emptying_of_pending_block() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();

        let initial_block_number = starknet.block_context.block_info().block_number;
        let initial_gas_price = starknet.block_context.block_info().gas_price;
        let initial_block_timestamp = starknet.block_context.block_info().block_timestamp;
        let initial_sequencer: ContractAddress =
            starknet.block_context.block_info().sequencer_address.clone().try_into().unwrap();

        // create pending block with some information in it
        let mut pending_block = StarknetBlock::create_pending_block();
        pending_block.add_transaction(crate::transactions::Transaction::Declare(
            dummy_declare_transaction_v1(),
        ));
        pending_block.status = BlockStatus::AcceptedOnL2;

        // assign the pending block
        starknet.blocks.pending_block = pending_block.clone();
        assert!(*starknet.pending_block() == pending_block);

        // empty the pending to block and check if it is in starting state
        starknet.restart_pending_block().unwrap();

        assert!(*starknet.pending_block() != pending_block);
        assert_eq!(starknet.pending_block().status, BlockStatus::Pending);
        assert!(starknet.pending_block().get_transactions().is_empty());
        assert_eq!(
            starknet.pending_block().header.timestamp,
            BlockTimestamp(initial_block_timestamp)
        );
        assert_eq!(starknet.pending_block().header.block_number, BlockNumber(initial_block_number));
        assert_eq!(starknet.pending_block().header.parent_hash, BlockHash::default());
        assert_eq!(starknet.pending_block().header.gas_price, GasPrice(initial_gas_price as u128));
        assert_eq!(
            starknet.pending_block().header.sequencer,
            initial_sequencer.try_into().unwrap()
        );
    }

    #[test]
    fn correct_block_context_update() {
        let mut block_ctx = Starknet::get_block_context(0, "0x0", StarknetChainId::TestNet).unwrap();
        let initial_block_number = block_ctx.block_info().block_number;
        Starknet::update_block_context(&mut block_ctx);

        assert_eq!(block_ctx.block_info().block_number, initial_block_number + 1);
    }
}
