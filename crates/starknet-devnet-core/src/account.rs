use std::fmt::Display;
use std::sync::Arc;

use blockifier::context::BlockContext;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::transactions::ExecutableTransaction;
use clap::Arg;
use starknet_api::core::calculate_contract_address;
use starknet_api::transaction::fields::{Calldata, ContractAddressSalt};
use starknet_api::{felt, patricia_key};
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::error::Error;
use starknet_types::felt::{ClassHash, Key, felt_from_prefixed_hex, join_felts, split_biguint};
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::state::Balance;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use starknet_types::rpc::transactions::{BroadcastedDeployAccountTransaction, BroadcastedTransactionCommonV3, ResourceBoundsWrapper};

use crate::constants::{
    CHARGEABLE_ACCOUNT_ADDRESS, CHARGEABLE_ACCOUNT_PRIVATE_KEY, CHARGEABLE_ACCOUNT_PUBLIC_KEY,
    ISRC6_ID_HEX,
};
use crate::contract_class_choice::{AccountClassWrapper, AccountContractClassChoice};
use crate::error::DevnetResult;
use crate::state::state_readers::DictState;
use crate::state::{CustomState, StarknetState};
use crate::traits::{Accounted, Deployed};
use crate::utils::get_storage_var_address;

/// data taken from https://github.com/0xSpaceShard/starknet-devnet-deprecated/blob/fb96e0cc3c1c31fb29892ecefd2a670cf8a32b51/starknet_devnet/account.py
const ACCOUNT_CLASS_HASH_HEX_FOR_ADDRESS_COMPUTATION: &str =
    "0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854";

pub enum FeeToken {
    ETH,
    STRK,
}

#[derive(Clone, Debug, Default, Copy)]
pub enum AccountType {
    OpenZeppelin_0_5_1,
    #[default]
    OpenZeppelin_0_20_0,
    Argent_0_4_0,
    Custom
}

impl Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let account_version = match self {
            AccountType::OpenZeppelin_0_5_1 => "OpenZeppelin 0.5.1",
            AccountType::OpenZeppelin_0_20_0 => "OpenZeppelin 0.20.0",
            AccountType::Argent_0_4_0 => "Argent 0.4.0",
            AccountType::Custom => "Custom",
        };

        f.write_str(account_version)
    }
}

#[derive(Clone)]
pub struct KeyPair {
    pub public_key: Key,
    pub private_key: Key,
}

#[derive(Clone)]
pub struct Account {
    pub keys: KeyPair,
    pub account_address: ContractAddress,
    pub initial_balance: Balance,
    pub class_hash: ClassHash,
    pub class_metadata: String,
    pub(crate) contract_class: ContractClass,
    pub(crate) eth_fee_token_address: ContractAddress,
    pub(crate) strk_fee_token_address: ContractAddress,
    block_context: BlockContext,
    account_type: AccountType,
    chain_id: Felt
}

impl Account {
    pub(crate) fn new_chargeable(
        eth_fee_token_address: ContractAddress,
        strk_fee_token_address: ContractAddress,
        block_context: BlockContext,
        account_type: AccountType,
        chain_id: Felt
    ) -> DevnetResult<Self> {
        let AccountClassWrapper { contract_class, class_hash, account_type } =
            AccountContractClassChoice::Cairo1.get_class_wrapper()?;

        // very big number
        let initial_balance = BigUint::from(u128::MAX) << 10;

        Ok(Self {
            keys: KeyPair {
                public_key: Key::from_hex(CHARGEABLE_ACCOUNT_PUBLIC_KEY)?,
                private_key: Key::from_hex(CHARGEABLE_ACCOUNT_PRIVATE_KEY)?,
            },
            account_address: ContractAddress::new(felt_from_prefixed_hex(
                CHARGEABLE_ACCOUNT_ADDRESS,
            )?)?,
            initial_balance,
            class_hash,
            class_metadata: account_type.to_string(),
            contract_class,
            eth_fee_token_address,
            strk_fee_token_address,
            block_context,
            account_type,
            chain_id
        })
    }

    pub(crate) fn new(
        initial_balance: Balance,
        keys: KeyPair,
        class_hash: ClassHash,
        contract_class: ContractClass,
        eth_fee_token_address: ContractAddress,
        strk_fee_token_address: ContractAddress,
        block_context: BlockContext,
        account_type: AccountType,
        chain_id: Felt
    ) -> DevnetResult<Self> {
        let account_address = Account::compute_account_address(&keys.public_key)?;
        Ok(Self {
            initial_balance,
            keys,
            class_hash,
            class_metadata: account_type.to_string(),
            contract_class,
            account_address,
            eth_fee_token_address,
            strk_fee_token_address,
            block_context,
            account_type,
            chain_id
        })
    }

    fn compute_account_address(public_key: &Key) -> DevnetResult<ContractAddress> {
        let account_address = calculate_contract_address(
            ContractAddressSalt(felt!(20u32)),
            starknet_api::core::ClassHash(felt_from_prefixed_hex(
                ACCOUNT_CLASS_HASH_HEX_FOR_ADDRESS_COMPUTATION,
            )?),
            &Calldata(Arc::new(vec![*public_key])),
            starknet_api::core::ContractAddress(patricia_key!(0u32)),
        )
        .map_err(Error::StarknetApiError)?;

        Ok(ContractAddress::from(account_address))
    }

    fn calldata(&self) -> Vec<Felt> {
        match self.account_type {
            AccountType::OpenZeppelin_0_5_1 |
            AccountType::OpenZeppelin_0_20_0 => vec![self.keys.public_key],
            AccountType::Argent_0_4_0 => todo!(),
            AccountType::Custom => vec![],
        }
    }

    // simulate constructor logic (register interfaces and set public key), as done in
    // https://github.com/OpenZeppelin/cairo-contracts/blob/89a450a88628ec3b86273f261b2d8d1ca9b1522b/src/account/account.cairo#L207-L211
    fn simulate_constructor(&self, state: &mut StarknetState) -> DevnetResult<()> {
        let core_address = self.account_address.try_into()?;
        let mut deploy_account_txn = BroadcastedDeployAccountTransaction::V3(
        BroadcastedDeployAccountTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce: Felt::ZERO,
                resource_bounds: ResourceBoundsWrapper::new(0,0,0,0,0,0),
                tip: Default::default(),
                paymaster_data: vec![],
                nonce_data_availability_mode: starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode: starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            contract_address_salt: Felt::ZERO,
            constructor_calldata: self.calldata(),
            class_hash: self.class_hash,
        }).create_sn_api_deploy_account(&self.chain_id)?;

        deploy_account_txn.contract_address = core_address;

        blockifier::transaction::account_transaction::AccountTransaction {
            tx: starknet_api::executable_transaction::AccountTransaction::DeployAccount(
                deploy_account_txn,
            ),
            execution_flags: ExecutionFlags { only_query: false, charge_fee: false, validate: false },
        }.execute(&mut state.state, &self.block_context)?;

        Ok(())
    }
}

impl Deployed for Account {
    fn deploy(&self, state: &mut StarknetState) -> DevnetResult<()> {
        self.declare_if_undeclared(state, self.class_hash, &self.contract_class)?;

        //state.predeploy_contract(self.account_address, self.class_hash)?;

        // set balance directly in the most underlying state
        self.set_initial_balance(&mut state.state.state)?;

        self.simulate_constructor(state)?;

        Ok(())
    }

    fn get_address(&self) -> ContractAddress {
        self.account_address
    }
}

impl Accounted for Account {
    fn set_initial_balance(&self, state: &mut DictState) -> DevnetResult<()> {
        let storage_var_address_low: starknet_api::state::StorageKey =
            get_storage_var_address("ERC20_balances", &[Felt::from(self.account_address)])?
                .try_into()?;

        let storage_var_address_high = storage_var_address_low.next_storage_key()?;

        let total_supply_storage_address_low: starknet_api::state::StorageKey =
            get_storage_var_address("ERC20_total_supply", &[])?.try_into()?;
        let total_supply_storage_address_high =
            total_supply_storage_address_low.next_storage_key()?;

        let (high, low) = split_biguint(self.initial_balance.clone());

        for fee_token_address in [self.eth_fee_token_address, self.strk_fee_token_address] {
            let token_address = fee_token_address.try_into()?;

            let total_supply_low =
                state.get_storage_at(token_address, total_supply_storage_address_low)?;
            let total_supply_high =
                state.get_storage_at(token_address, total_supply_storage_address_high)?;

            let new_total_supply =
                join_felts(&total_supply_high, &total_supply_low) + self.initial_balance.clone();

            let (new_total_supply_high, new_total_supply_low) = split_biguint(new_total_supply);

            // set balance in ERC20_balances
            state.set_storage_at(token_address, storage_var_address_low, low)?;
            state.set_storage_at(token_address, storage_var_address_high, high)?;

            // set total supply in ERC20_total_supply
            state.set_storage_at(
                token_address,
                total_supply_storage_address_low,
                new_total_supply_low,
            )?;

            state.set_storage_at(
                token_address,
                total_supply_storage_address_high,
                new_total_supply_high,
            )?;
        }

        Ok(())
    }

    fn get_balance(&self, state: &mut impl StateReader, token: FeeToken) -> DevnetResult<Balance> {
        let fee_token_address = match token {
            FeeToken::ETH => self.eth_fee_token_address,
            FeeToken::STRK => self.strk_fee_token_address,
        };
        let (low, high) = state.get_fee_token_balance(
            self.account_address.try_into()?,
            fee_token_address.try_into()?,
        )?;
        Ok(join_felts(&high, &low))
    }
}

#[cfg(test)]
mod tests {
    use blockifier::context::{BlockContext, ChainInfo};
    use blockifier::versioned_constants::VersionedConstants;
    use starknet_api::block::BlockInfo;
    use starknet_rs_core::types::Felt;
    use starknet_types::chain_id::ChainId;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::rpc::state::Balance;

    use super::{Account, KeyPair};
    use crate::account::FeeToken;
    use crate::constants::{CAIRO_1_ERC20_CONTRACT_CLASS_HASH, USE_KZG_DA};
    use crate::starknet::Starknet;
    use crate::state::{CustomState, StarknetState};
    use crate::traits::{Accounted, Deployed};
    use crate::utils::{custom_bouncer_config, get_versioned_constants};
    use crate::utils::test_utils::{
        cairo_0_account_without_validations, dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt
    };

    /// Testing if generated account address has the same value as the first account in
    /// https://github.com/0xSpaceShard/starknet-devnet-deprecated/blob/9d867e38e6d465e568e82a47e82e40608f6d220f/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn account_address_should_be_equal() {
        let expected_result = ContractAddress::new(
            felt_from_prefixed_hex(
                "0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502",
            )
            .unwrap(),
        )
        .unwrap();
        let generated_result = Account::compute_account_address(
            &felt_from_prefixed_hex(
                "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b5",
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn account_address_should_not_be_equal() {
        let expected_result = ContractAddress::new(
            felt_from_prefixed_hex(
                "0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502",
            )
            .unwrap(),
        )
        .unwrap();
        let generated_result = Account::compute_account_address(
            &felt_from_prefixed_hex(
                "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b6",
            )
            .unwrap(),
        )
        .unwrap();

        assert_ne!(expected_result, generated_result);
    }

    #[test]
    fn account_deployed_successfully() {
        let (account, mut state) = setup();
        assert!(account.deploy(&mut state).is_ok());
    }

    #[test]
    fn account_get_balance_should_return_correct_value() {
        let (mut account, mut state) = setup();
        let expected_balance = Balance::from(100_u8);
        account.initial_balance = expected_balance.clone();
        account.deploy(&mut state).unwrap();
        let generated_balance = account.get_balance(&mut state, FeeToken::ETH).unwrap();

        assert_eq!(expected_balance, generated_balance);

        let generated_balance = account.get_balance(&mut state, FeeToken::STRK).unwrap();

        assert_eq!(expected_balance, generated_balance);
    }

    #[test]
    fn account_changed_balance_successfully_without_deployment() {
        let (account, mut state) = setup();
        assert!(account.set_initial_balance(&mut state.state.state).is_ok());
    }

    #[test]
    fn account_get_address_correct() {
        let (mut account, _) = setup();
        let expected_address = ContractAddress::new(Felt::from(11111)).unwrap();
        account.account_address = expected_address;
        assert_eq!(expected_address, account.get_address());
    }

    fn setup() -> (Account, StarknetState) {
        let mut state = StarknetState::default();
        let fee_token_address = dummy_contract_address();
        let block_context = BlockContext::new(BlockInfo::create_for_testing_with_kzg(USE_KZG_DA), ChainInfo::default(), get_versioned_constants(), custom_bouncer_config());

        let account_contract_class = cairo_0_account_without_validations();
        // deploy the erc20 contract
        state.predeploy_contract(fee_token_address, CAIRO_1_ERC20_CONTRACT_CLASS_HASH).unwrap();

        (
            Account::new(
                Balance::from(10_u8),
                KeyPair { public_key: Felt::from(13431515), private_key: Felt::from(11) },
                dummy_felt(),
                account_contract_class.into(),
                fee_token_address,
                fee_token_address,
                block_context,
                super::AccountType::Custom,
                ChainId::Testnet.to_felt()
            )
            .unwrap(),
            state,
        )
    }
}
