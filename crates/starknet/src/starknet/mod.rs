use std::collections::HashMap;
use std::time::SystemTime;

use starknet_api::block::{BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
use starknet_in_rust::call_contract;
use starknet_in_rust::definitions::block_context::{
    BlockContext, StarknetChainId, StarknetOsConfig,
};
use starknet_in_rust::definitions::constants::{
    DEFAULT_CAIRO_RESOURCE_FEE_WEIGHTS, DEFAULT_CONTRACT_STORAGE_COMMITMENT_TREE_HEIGHT,
    DEFAULT_GLOBAL_STATE_COMMITMENT_TREE_HEIGHT, DEFAULT_INVOKE_TX_MAX_N_STEPS,
    DEFAULT_VALIDATE_MAX_N_STEPS,
};
use starknet_in_rust::execution::TransactionExecutionInfo;
use starknet_in_rust::state::state_api::State;
use starknet_in_rust::state::BlockInfo;
use starknet_in_rust::testing::TEST_SEQUENCER_ADDRESS;
use starknet_in_rust::utils::Address;
use starknet_rs_core::types::{BlockId, TransactionStatus};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_ff::FieldElement;
use starknet_rs_signers::Signer;
use tracing::error;

use self::predeployed::initialize_erc20;
use crate::account::Account;
use crate::blocks::{StarknetBlock, StarknetBlocks};
use crate::constants::{CAIRO_0_ACCOUNT_CONTRACT_PATH, ERC20_CONTRACT_ADDRESS};
use crate::constants::{CHARGEABLE_ACCOUNT_ADDRESS, CHARGEABLE_ACCOUNT_PRIVATE_KEY};
use crate::error::{Error, Result};
use crate::predeployed_accounts::PredeployedAccounts;
use crate::raw_execution::{Call, RawExecution};
use crate::state::state_diff::StateDiff;
use crate::state::state_update::StateUpdate;
use crate::state::StarknetState;
use crate::traits::{
    AccountGenerator, Accounted, Deployed, HashIdentifiedMut, StateChanger, StateExtractor,
};
use crate::transactions::declare_transaction::DeclareTransactionV1;
use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::transactions::invoke_transaction::InvokeTransactionV1;
use crate::transactions::{StarknetTransaction, StarknetTransactions, Transaction};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0ContractClass, Cairo0Json, ContractClass};
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::patricia_key::PatriciaKey;
use starknet_types::traits::HashProducer;

mod add_declare_transaction;
mod add_deploy_account_transaction;
mod add_invoke_transaction;
mod get_class_impls;
mod predeployed;
mod state_update;

#[derive(Clone, Debug)]
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
    pub transactions: StarknetTransactions,
    pub config: StarknetConfig,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> Result<Self> {
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
        let account_contract_class = Cairo0Json::raw_json_from_path(CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
        let class_hash = account_contract_class.generate_hash()?;

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            class_hash,
            account_contract_class.clone().into(),
        )?;
        for account in accounts {
            account.deploy(&mut state)?;
            account.set_initial_balance(&mut state)?;
        }

        let chargeable_account = Account::new_chargeable(
            class_hash,
            account_contract_class.into(),
            erc20_fee_contract.get_address(),
        );
        chargeable_account.deploy(&mut state)?;

        // copy already modified state to cached state
        state.synchronize_states();

        let mut this = Self {
            state,
            predeployed_accounts,
            block_context: Self::get_block_context(0, ERC20_CONTRACT_ADDRESS, config.chain_id)?,
            blocks: StarknetBlocks::default(),
            transactions: StarknetTransactions::default(),
            config: config.clone(),
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

    /// Transfer data from pending block into new block and save it to blocks collection
    /// Returns the new block number
    pub(crate) fn generate_new_block(
        &mut self,
        state_diff: StateDiff,
        state: StarknetState,
    ) -> Result<BlockNumber> {
        let mut new_block = self.pending_block().clone();

        // set new block header
        new_block.set_block_hash(new_block.generate_hash()?);
        new_block.status = BlockStatus::AcceptedOnL2;
        let new_block_number = new_block.block_number();

        // update txs block hash block number for each transaction in the pending block
        new_block.get_transactions().iter().for_each(|t| {
            if let Some(tx_hash) = t.get_hash() {
                if let Some(tx) = self.transactions.get_by_hash_mut(&tx_hash) {
                    tx.block_hash = Some(new_block.header.block_hash.0.into());
                    tx.block_number = Some(new_block_number);
                    tx.status = TransactionStatus::AcceptedOnL2;
                } else {
                    error!("Transaction is not present in the transactions colletion");
                }
            } else {
                error!("Transaction has no generated hash");
            }
        });

        // insert pending block in the blocks collection and connect it to the state diff
        self.blocks.insert(new_block, state_diff);
        // save into blocks state archive
        self.blocks.save_state_at(new_block_number, state);

        Ok(new_block_number)
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

        let state_difference = self.state.extract_state_diff_from_pending_state()?;
        // apply state changes from cached state
        self.state.apply_state_difference(state_difference.clone())?;
        // make cached state part of "persistent" state
        self.state.synchronize_states();
        // create new block from pending one
        self.generate_new_block(state_difference, self.state.clone())?;
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

    pub(crate) fn get_state_at_mut(&mut self, block_id: &BlockId) -> Result<&mut StarknetState> {
        match block_id {
            BlockId::Tag(_) => Ok(&mut self.state),
            _ => {
                let block = self.blocks.get_by_block_id(*block_id).ok_or(Error::NoBlock)?.clone();
                let state = self
                    .blocks
                    .num_to_state
                    .get_mut(&block.block_number())
                    .ok_or(Error::NoStateAtBlock { block_number: block.block_number().0 })?;
                Ok(state)
            }
        }
    }

    pub fn get_class_hash_at(
        &mut self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> Result<ClassHash> {
        get_class_impls::get_class_hash_at_impl(self, block_id, contract_address)
    }

    pub fn get_class(&mut self, block_id: BlockId, class_hash: ClassHash) -> Result<ContractClass> {
        get_class_impls::get_class_impl(self, block_id, class_hash)
    }

    pub fn get_class_at(
        &mut self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> Result<ContractClass> {
        get_class_impls::get_class_at_impl(self, block_id, contract_address)
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
            // dummy caller_address since there is no account address; safe to unwrap since it's
            // just 0
            ContractAddress::zero().try_into().unwrap(),
        )?;
        Ok(result.iter().map(|e| Felt::from(e.clone())).collect())
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

    /// returning the chain id as object
    pub fn chain_id(&self) -> StarknetChainId {
        self.config.chain_id
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

    pub fn add_invoke_transaction_v1(
        &mut self,
        invoke_transaction: InvokeTransactionV1,
    ) -> Result<TransactionHash> {
        add_invoke_transaction::add_invoke_transcation_v1(self, invoke_transaction)
    }

    /// Creates an invoke tx for minting, using the chargeable account.
    pub async fn mint(&mut self, address: ContractAddress, amount: u128) -> Result<Felt> {
        let sufficiently_big_max_fee: u128 = self.config.gas_price as u128 * 1_000_000;
        let chargeable_address_felt = Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS)?;
        let nonce =
            self.state.pending_state.get_nonce_at(&Address(chargeable_address_felt.into()))?;

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
            nonce: Felt::from(nonce.clone()).into(),
            max_fee: FieldElement::from(sufficiently_big_max_fee),
        };

        // generate msg hash (not the same as tx hash)
        let chain_id_felt: Felt = self.config.chain_id.to_felt().into();
        let msg_hash_felt =
            raw_execution.transaction_hash(chain_id_felt.into(), chargeable_address_felt.into());

        // generate signature by signing the msg hash
        let signer = starknet_rs_signers::LocalWallet::from(
            starknet_rs_signers::SigningKey::from_secret_scalar(
                FieldElement::from_hex_be(CHARGEABLE_ACCOUNT_PRIVATE_KEY).unwrap(),
            ),
        );
        let signature = signer.sign_hash(&msg_hash_felt).await?;

        // apply the invoke tx
        let invoke_tx = InvokeTransactionV1::new(
            ContractAddress::new(chargeable_address_felt)?,
            sufficiently_big_max_fee,
            vec![signature.r.into(), signature.s.into()],
            nonce.into(),
            raw_execution.raw_calldata().into_iter().map(|c| c.into()).collect(),
            chain_id_felt,
        )?;
        self.add_invoke_transaction_v1(invoke_tx)
    }

    pub fn block_state_update(&self, block_id: BlockId) -> Result<StateUpdate> {
        state_update::state_update_by_block_id(self, block_id)
    }

    pub fn get_block_txs_count(&self, block_id: BlockId) -> Result<u64> {
        let block = self.blocks.get_by_block_id(block_id).ok_or(Error::NoBlock)?;

        Ok(block.get_transactions().len() as u64)
    }

    pub fn contract_nonce_at_block(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
    ) -> Result<Felt> {
        let state = self.get_state_at(&block_id)?;
        match state.state.address_to_nonce.get(&contract_address.try_into()?) {
            Some(nonce) => Ok(Felt::from(nonce.clone())),
            None => Err(Error::ContractNotFound),
        }
    }

    pub fn contract_storage_at_block(
        &self,
        block_id: BlockId,
        contract_address: ContractAddress,
        storage_key: PatriciaKey,
    ) -> Result<Felt> {
        let state = self.get_state_at(&block_id)?;
        state.get_storage(ContractStorageKey::new(contract_address, storage_key))
    }

    pub fn get_block(&self, block_id: BlockId) -> Result<StarknetBlock> {
        let block = self.blocks.get_by_block_id(block_id).ok_or(crate::error::Error::NoBlock)?;
        Ok(block.clone())
    }

    pub fn get_latest_block(&self) -> Result<StarknetBlock> {
        let block = self
            .blocks
            .get_by_block_id(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest))
            .ok_or(crate::error::Error::NoBlock)?;

        Ok(block.clone())
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::{BlockHash, BlockNumber, BlockStatus, BlockTimestamp, GasPrice};
    use starknet_in_rust::core::errors::state_errors::StateError;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::felt::Felt252;
    use starknet_in_rust::transaction::error::TransactionError;
    use starknet_in_rust::utils::Address;
    use starknet_rs_core::types::{BlockId, BlockTag};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use super::Starknet;
    use crate::blocks::StarknetBlock;
    use crate::constants::{
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_INITIAL_BALANCE, ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::{Error, Result};
    use crate::state::state_diff::StateDiff;
    use crate::traits::{Accounted, StateChanger, StateExtractor};
    use crate::utils::test_utils::{
        dummy_contract_address, dummy_declare_transaction_v1, starknet_config_for_test,
    };

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

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
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
    fn getting_state_of_latest_block() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Latest)).expect("Should be OK");
    }

    #[test]
    fn getting_state_of_pending_block() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        starknet.get_state_at(&BlockId::Tag(BlockTag::Pending)).expect("Should be OK");
    }

    #[test]
    fn getting_state_at_block_by_nonexistent_hash() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();

        match starknet.get_state_at(&BlockId::Hash(Felt::from(0).into())) {
            Err(Error::NoBlock) => (),
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn getting_nonexistent_state_at_block_by_number() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();
        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.blocks.num_to_state.remove(&BlockNumber(0));

        match starknet.get_state_at(&BlockId::Number(0)) {
            Err(Error::NoStateAtBlock { block_number: _ }) => (),
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

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        // last added block number -> 0
        let added_block = starknet.blocks.num_to_block.get(&BlockNumber(0)).unwrap();
        // number of the accepted block -> 1
        let block_number = starknet.block_number();

        assert_eq!(block_number.0 - 1, added_block.header.block_number.0);

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        let added_block2 = starknet.blocks.num_to_block.get(&BlockNumber(1)).unwrap();
        let block_number2 = starknet.block_number();

        assert_eq!(block_number2.0 - 1, added_block2.header.block_number.0);
    }

    #[test]
    fn gets_block_txs_count() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        let num_no_transactions = starknet.get_block_txs_count(BlockId::Number(0));

        assert_eq!(num_no_transactions.unwrap(), 0);

        let mut tx = dummy_declare_transaction_v1();
        let tx_hash = tx.generate_hash().unwrap();
        tx.transaction_hash = Some(tx_hash);

        // add transaction to pending block
        starknet
            .blocks
            .pending_block
            .add_transaction(crate::transactions::Transaction::Declare(tx));

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();

        let num_one_transaction = starknet.get_block_txs_count(BlockId::Number(1));

        assert_eq!(num_one_transaction.unwrap(), 1);
    }

    #[test]
    fn returns_chain_id() {
        let config = starknet_config_for_test();
        let starknet = Starknet::new(&config).unwrap();
        let chain_id = starknet.chain_id();

        assert_eq!(chain_id.to_string(), DEVNET_DEFAULT_CHAIN_ID.to_string());
    }

    #[test]
    fn correct_state_at_specific_block() {
        let mut starknet = Starknet::default();
        // generate initial block with empty state
        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        // **generate second block**
        // add data to state
        starknet
            .state
            .pending_state
            .cache_mut()
            .nonce_writes_mut()
            .insert(dummy_contract_address().try_into().unwrap(), Felt::from(1).into());
        // get state difference
        let state_diff = starknet.state.extract_state_diff_from_pending_state().unwrap();
        // move data from pending_state to state
        starknet.state.apply_state_difference(state_diff.clone()).unwrap();
        // generate new block and save the state
        let second_block = starknet.generate_new_block(state_diff, starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        // **generate third block**
        // add data to state
        starknet
            .state
            .pending_state
            .cache_mut()
            .nonce_writes_mut()
            .insert(dummy_contract_address().try_into().unwrap(), Felt::from(2).into());
        // get state difference
        let state_diff = starknet.state.extract_state_diff_from_pending_state().unwrap();
        // move data from pending_state to state
        starknet.state.apply_state_difference(state_diff.clone()).unwrap();
        // generate new block and save the state
        let third_block = starknet.generate_new_block(state_diff, starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        // check modified state at block 1 and 2 to contain the correct value for the nonce
        let second_block_address_nonce = starknet
            .blocks
            .num_to_state
            .get(&second_block)
            .unwrap()
            .state
            .address_to_nonce
            .get(&dummy_contract_address().try_into().unwrap())
            .unwrap();
        let second_block_expected_address_nonce = Felt252::from(1);
        assert_eq!(second_block_expected_address_nonce, *second_block_address_nonce);

        let third_block_address_nonce = starknet
            .blocks
            .num_to_state
            .get(&third_block)
            .unwrap()
            .state
            .address_to_nonce
            .get(&dummy_contract_address().try_into().unwrap())
            .unwrap();
        let third_block_expected_address_nonce = Felt252::from(2);
        assert_eq!(third_block_expected_address_nonce, *third_block_address_nonce);
    }

    #[test]
    fn gets_latest_block() {
        let config = starknet_config_for_test();
        let mut starknet = Starknet::new(&config).unwrap();

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();
        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();
        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();

        let latest_block = starknet.get_latest_block();

        assert_eq!(latest_block.unwrap().block_number(), BlockNumber(2));
    }
}
