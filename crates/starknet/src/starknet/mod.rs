use std::collections::HashMap;
use std::time::SystemTime;

use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
use starknet_in_rust::definitions::block_context::{
    BlockContext, StarknetChainId, StarknetOsConfig,
};
use starknet_in_rust::definitions::constants::{
    DEFAULT_CAIRO_RESOURCE_FEE_WEIGHTS, DEFAULT_CONTRACT_STORAGE_COMMITMENT_TREE_HEIGHT,
    DEFAULT_GLOBAL_STATE_COMMITMENT_TREE_HEIGHT, DEFAULT_INVOKE_TX_MAX_N_STEPS,
    DEFAULT_VALIDATE_MAX_N_STEPS,
};
use starknet_in_rust::execution::TransactionExecutionInfo;
use starknet_in_rust::state::BlockInfo;
use starknet_in_rust::testing::TEST_SEQUENCER_ADDRESS;
use starknet_in_rust::utils::Address;
use starknet_in_rust::{call_contract, SierraContractClass};
use starknet_rs_core::types::{BlockId, TransactionStatus};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use tracing::error;

use crate::account::Account;
use crate::blocks::{StarknetBlock, StarknetBlocks};
use crate::constants::{CAIRO_0_ACCOUNT_CONTRACT_PATH, ERC20_CONTRACT_ADDRESS};
use crate::error::{Error, Result};
use crate::predeployed_accounts::PredeployedAccounts;
use crate::state::StarknetState;
use crate::traits::{AccountGenerator, Accounted, HashIdentifiedMut, StateChanger};
use crate::transactions::declare_transaction::DeclareTransactionV1;
use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::transactions::{StarknetTransaction, StarknetTransactions, Transaction};
use crate::utils;

mod add_declare_transaction;
mod add_deploy_account_transaction;
mod predeployed;

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

impl Default for StarknetConfig {
    fn default() -> Self {
        Self {
            seed: u32::default(),
            total_accounts: u8::default(),
            predeployed_accounts_initial_balance: Felt::default(),
            host: String::default(),
            port: u16::default(),
            timeout: u16::default(),
            gas_price: u64::default(),
            chain_id: StarknetChainId::TestNet,
        }
    }
}

#[derive(Default)]
pub struct Starknet {
    pub(in crate::starknet) state: StarknetState,
    predeployed_accounts: PredeployedAccounts,
    pub(in crate::starknet) block_context: BlockContext,
    blocks: StarknetBlocks,
    transactions: StarknetTransactions,
    pub config: StarknetConfig,
    pub(in crate::starknet) sierra_contracts: HashMap<ClassHash, SierraContractClass>,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> Result<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_fee_contract = predeployed::create_erc20()?;
        let udc_contract = predeployed::create_udc20()?;

        erc20_fee_contract.deploy(&mut state)?;
        udc_contract.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance,
            erc20_fee_contract.get_address(),
        );
        let account_contract_class =
            utils::load_cairo_0_contract_class(CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
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
            config: StarknetConfig::default(),
            sierra_contracts: HashMap::new(),
        };

        this.restart_pending_block()?;

        Ok(this)
    }

    pub fn get_predeployed_accounts(&self) -> Vec<Account> {
        self.predeployed_accounts.get_accounts().to_vec()
    }

    // Update block context
    // Initialize values for new pending block
    pub(crate) fn generate_pending_block(&mut self) -> Result<()> {
        Self::update_block_context(&mut self.block_context);
        self.restart_pending_block()?;

        Ok(())
    }

    // Transfer data from pending block into new block and save it to blocks collection
    pub(crate) fn generate_new_block(&mut self) -> Result<()> {
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
    ) -> Result<()> {
        let transaction_to_add =
            StarknetTransaction::create_successful(transaction.clone(), tx_info);

        // add accepted transaction to pending block
        self.blocks.pending_block.add_transaction(transaction);

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

    fn get_block_context(
        gas_price: u64,
        fee_token_address: &str,
        chain_id: StarknetChainId,
    ) -> Result<BlockContext> {
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
    fn restart_pending_block(&mut self) -> Result<()> {
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

    fn get_state_at(&self, block_id: &BlockId) -> Result<&StarknetState> {
        match block_id {
            BlockId::Tag(_) => Ok(&self.state),
            BlockId::Hash(_) => Err(Error::BlockIdHashUnimplementedError),
            BlockId::Number(_) => Err(Error::BlockIdNumberUnimplementedError),
        }
    }

    pub fn get_class_hash_at(
        &self,
        block_id: &BlockId,
        contract_address: &ContractAddress,
    ) -> Result<Felt> {
        let state = self.get_state_at(block_id)?;
        let address: Address = contract_address.try_into()?;
        match state.state.address_to_class_hash.get(&address) {
            Some(class_hash) => Ok(Felt::from(*class_hash)),
            None => Err(Error::ContractNotFound),
        }
    }

    pub fn call(
        &self,
        block_id: BlockId,
        contract_address: Felt,
        entrypoint_selector: Felt,
        calldata: Vec<Felt>,
    ) -> Result<Vec<Felt>> {
        let state = self.get_state_at(&block_id)?;

        let result = call_contract(
            contract_address.into(),
            entrypoint_selector.into(),
            calldata.iter().map(|c| c.into()).collect(),
            &mut state.pending_state.clone(),
            self.block_context.clone(),
            // dummy caller_address since there is no account address
            ContractAddress::zero().try_into()?,
        )?;
        Ok(result.iter().map(|e| Felt::from(e.clone())).collect())
    }

    pub fn estimate_fee(
        &self,
        block_id: BlockId,
        transactions: &[starknet_in_rust::transaction::Transaction],
        // TODO define a better return type
    ) -> Result<Vec<(u128, usize)>> {
        let state = self.get_state_at(&block_id)?;

        // Vec<(Fee, GasUsage)>
        Ok(starknet_in_rust::estimate_fee(
            transactions,
            state.pending_state.clone(),
            &self.block_context,
        )?)
    }

    pub fn add_declare_transaction_v1(
        &mut self,
        declare_transaction: DeclareTransactionV1,
    ) -> Result<(TransactionHash, ClassHash)> {
        add_declare_transaction::add_declare_transaction_v1(self, declare_transaction)
    }

    pub fn add_declare_transaction_v2(
        &mut self,
        declare_transaction: DeclareTransactionV2,
    ) -> Result<(TransactionHash, ClassHash)> {
        add_declare_transaction::add_declare_transaction_v2(self, declare_transaction)
    }

    /// returning the block number that will be added, ie. the most recent accepted block number
    pub fn block_number(&self) -> BlockNumber {
        let block_num: u64 = self.block_context.block_info().block_number;
        BlockNumber(block_num)
    }

    pub fn add_deploy_account_transaction(
        &mut self,
        deploy_account_transaction: DeployAccountTransaction,
    ) -> Result<(TransactionHash, ContractAddress)> {
        add_deploy_account_transaction::add_deploy_account_transaction(
            self,
            deploy_account_transaction,
        )
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{BlockHash, BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
    use starknet_in_rust::core::errors::state_errors::StateError;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::transaction::error::TransactionError;
    use starknet_in_rust::utils::Address;
    use starknet_rs_core::types::{BlockId, BlockTag};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use super::Starknet;
    use crate::blocks::StarknetBlock;
    use crate::constants::{DEVNET_DEFAULT_INITIAL_BALANCE, ERC20_CONTRACT_ADDRESS};
    use crate::error::{Error, Result};
    use crate::traits::Accounted;
    use crate::utils::test_utils::{dummy_declare_transaction_v1, starknet_config_for_test};

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
        let mut block_ctx =
            Starknet::get_block_context(0, "0x0", StarknetChainId::TestNet).unwrap();
        let initial_block_number = block_ctx.block_info().block_number;
        Starknet::update_block_context(&mut block_ctx);

        assert_eq!(block_ctx.block_info().block_number, initial_block_number + 1);
    }

    #[test]
    fn getting_state_reader_of_latest_state() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Latest)).expect("Should be OK");
    }

    #[test]
    fn getting_state_reader_of_pending_state() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Pending)).expect("Should be OK");
    }

    #[test]
    fn getting_state_reader_at_block_by_hash() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        match starknet.get_state_at(&BlockId::Number(2)) {
            Err(Error::BlockIdNumberUnimplementedError) => (),
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn getting_state_reader_at_block_by_number() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        match starknet.get_state_at(&BlockId::Number(2)) {
            Err(Error::BlockIdNumberUnimplementedError) => (),
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn calling_method_of_undeployed_contract() {
        let config = starknet_config_for_test();
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
            Err(Error::TransactionError(TransactionError::State(
                StateError::NoneContractState(Address(address)),
            ))) => {
                let received_address_hex = format!("0x{}", address.to_str_radix(16));
                assert_eq!(received_address_hex.as_str(), undeployed_address_hex);
            }
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    #[test]
    fn calling_nonexistent_contract_method() {
        let config = starknet_config_for_test();
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
            Err(Error::TransactionError(TransactionError::EntryPointNotFound)) => (),
            unexpected => panic!("Should have failed; got {unexpected:?}"),
        }
    }

    /// utility method for happy path balance retrieval
    fn get_balance_at(starknet: &Starknet, contract_address: ContractAddress) -> Result<Vec<Felt>> {
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
        let config = starknet_config_for_test();
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
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();

        let undeployed_address =
            ContractAddress::new(Felt::from_prefixed_hex_str("0x1234").unwrap()).unwrap();
        let result = get_balance_at(&starknet, undeployed_address).unwrap();

        let zero = Felt::from_prefixed_hex_str("0x0").unwrap();
        let expected_balance_uint256 = vec![zero, zero];
        assert_eq!(result, expected_balance_uint256);
    }

    #[test]
    fn returns_block_number() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();

        let block_number_no_blocks = starknet.block_number();
        assert_eq!(block_number_no_blocks.0, 0);

        starknet.generate_new_block().unwrap();
        starknet.generate_pending_block().unwrap();

        // last added block number -> 0
        let added_block = starknet.blocks.num_to_block.get(&BlockNumber(0)).unwrap();
        // number of the accepted block -> 1
        let block_number = starknet.block_number();

        assert_eq!(block_number.0 - 1, added_block.header.block_number.0);

        starknet.generate_new_block().unwrap();
        starknet.generate_pending_block().unwrap();

        let added_block2 = starknet.blocks.num_to_block.get(&BlockNumber(1)).unwrap();
        let block_number2 = starknet.block_number();

        assert_eq!(block_number2.0 - 1, added_block2.header.block_number.0);
    }
}
