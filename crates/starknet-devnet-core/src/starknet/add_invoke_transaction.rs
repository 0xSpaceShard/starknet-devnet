use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::invoke_transaction_v3::InvokeTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedInvokeTransaction, InvokeTransaction, Transaction, TransactionWithHash,
};

use super::Starknet;
use crate::error::{DevnetResult, Error, TransactionValidationError};

pub fn add_invoke_transaction(
    starknet: &mut Starknet,
    broadcasted_invoke_transaction: BroadcastedInvokeTransaction,
) -> DevnetResult<TransactionHash> {
    if !broadcasted_invoke_transaction.are_gas_bounds_valid() {
        return Err(TransactionValidationError::InsufficientResourcesForValidate.into());
    }

    if broadcasted_invoke_transaction.is_only_query() {
        return Err(Error::UnsupportedAction {
            msg: "only-query transactions are not supported".to_string(),
        });
    }

    let sn_api_transaction =
        broadcasted_invoke_transaction.create_sn_api_invoke(&starknet.chain_id().to_felt())?;

    let transaction_hash = sn_api_transaction.tx_hash.0;

    let invoke_transaction = match broadcasted_invoke_transaction {
        BroadcastedInvokeTransaction::V3(ref v3) => {
            Transaction::Invoke(InvokeTransaction::V3(InvokeTransactionV3::new(v3)))
        }
    };

    let validate = !(Starknet::is_account_impersonated(
        &mut starknet.pending_state,
        &starknet.cheats,
        &ContractAddress::from(sn_api_transaction.sender_address()),
    )?);

    let block_context = starknet.block_context.clone();

    let state = &mut starknet.get_state().state;

    let execution_info = blockifier::transaction::account_transaction::AccountTransaction {
        tx: starknet_api::executable_transaction::AccountTransaction::Invoke(sn_api_transaction),
        execution_flags: ExecutionFlags { only_query: false, charge_fee: true, validate },
    }
    .execute(state, &block_context);

    let execution_info = execution_info?;

    let transaction = TransactionWithHash::new(transaction_hash, invoke_transaction);

    starknet.handle_accepted_transaction(transaction, execution_info)?;

    Ok(transaction_hash)
}
#[cfg(test)]
mod tests {
    use blockifier::state::state_api::StateReader;
    use nonzero_ext::nonzero;
    use starknet_api::core::Nonce;
    use starknet_rs_core::types::{Felt, TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::num_bigint::BigUint;
    use starknet_types::rpc::gas_modification::GasModification;
    use starknet_types::rpc::state::Balance;
    use starknet_types::rpc::transactions::BroadcastedInvokeTransaction;
    use starknet_types::traits::HashProducer;

    use crate::account::{Account, FeeToken};
    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        ETH_ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::{Error, TransactionValidationError};
    use crate::starknet::{Starknet, predeployed};
    use crate::state::CustomState;
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::get_storage_var_address;
    use crate::utils::test_utils::{
        cairo_0_account_without_validations, dummy_contract_address, dummy_felt, dummy_key_pair,
        resource_bounds_with_price_1, test_invoke_transaction_v3,
    };

    #[test]
    fn invoke_transaction_v3_with_only_query_version_should_return_an_error() {
        let mut invoke_transaction = test_invoke_transaction_v3(
            dummy_contract_address(),
            dummy_contract_address(),
            dummy_felt(),
            &[Felt::from(10)],
            0, // nonce
            resource_bounds_with_price_1(1, 0, 0),
        );

        let BroadcastedInvokeTransaction::V3(ref mut tx_v3) = invoke_transaction;
        tx_v3.common.version = Felt::THREE + QUERY_VERSION_OFFSET;

        let result = Starknet::default().add_invoke_transaction(invoke_transaction);
        match result {
            Err(crate::error::Error::UnsupportedAction { msg }) => {
                assert_eq!(msg, "only-query transactions are not supported")
            }
            other => panic!("Unexpected result: {other:?}"),
        };
    }

    fn biguint_to_u64(b: &BigUint) -> u64 {
        let parts = b.to_u64_digits();
        assert_eq!(parts.len(), 1);
        parts[0]
    }

    #[test]
    fn invoke_transaction_v3_successful_execution_with_only_l1_gas() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();
        let account_address = account.get_address();
        let initial_balance =
            account.get_balance(&mut starknet.pending_state, FeeToken::STRK).unwrap();

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            &[Felt::from(10)],
            0, // nonce
            resource_bounds_with_price_1(biguint_to_u64(&initial_balance), 0, 0),
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();

        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(
            account.get_balance(&mut starknet.pending_state, FeeToken::STRK).unwrap()
                < initial_balance
        );
    }

    #[test]
    fn invoke_transaction_v3_successful_execution_with_all_three_gas_bounds() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();
        let account_address = account.get_address();
        let initial_balance =
            account.get_balance(&mut starknet.pending_state, FeeToken::STRK).unwrap();

        // dividing by 10, otherwise it fails with gas exceeding user balance
        let gas_amount = biguint_to_u64(&initial_balance) / 10;

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            &[Felt::from(10)],
            0,
            resource_bounds_with_price_1(gas_amount, gas_amount, gas_amount),
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();

        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(
            account.get_balance(&mut starknet.pending_state, FeeToken::STRK).unwrap()
                < initial_balance
        );
    }

    #[test]
    fn invoke_transaction_v3_with_invalid_gas_amounts() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();
        let account_address = account.get_address();

        let balance: u64 = account
            .get_balance(&mut starknet.pending_state, FeeToken::STRK)
            .unwrap()
            .to_string()
            .parse()
            .unwrap();
        assert!(balance > 0);

        // either only l1_gas is allowed or all three must be set, otherwise invalid
        for (l1_gas, l1_data_gas, l2_gas) in
            [(balance, 0, 1), (0, 0, 1), (0, balance, 0), (balance, balance, 0), (0, 0, 0)]
        {
            let invoke_transaction = test_invoke_transaction_v3(
                account_address,
                contract_address,
                increase_balance_selector,
                &[Felt::from(10)],
                0,
                resource_bounds_with_price_1(l1_gas, l1_data_gas, l2_gas),
            );

            match starknet.add_invoke_transaction(invoke_transaction) {
                Err(Error::TransactionValidationError(
                    TransactionValidationError::InsufficientResourcesForValidate,
                )) => {}
                other => {
                    panic!("Wrong result: {other:?}")
                }
            }
        }
    }

    #[test]
    fn invoke_transaction_v1_successfully_changes_storage() {
        let (
            mut starknet,
            account,
            contract_address,
            increase_balance_selector,
            balance_var_storage_address,
        ) = setup();
        let blockifier_address = contract_address.try_into().unwrap();
        let storage_key = (*balance_var_storage_address.get_storage_key()).try_into().unwrap();

        let account_address = account.get_address();
        let resource_bounds = resource_bounds_with_price_1(0, 1000, 1e6 as u64);

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            &[Felt::from(10)],
            0, // nonce
            resource_bounds.clone(),
        );

        // invoke transaction
        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check storage
        assert_eq!(
            starknet.pending_state.get_storage_at(blockifier_address, storage_key).unwrap(),
            Felt::from(10)
        );

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            &[Felt::from(15)],
            1, // nonce
            resource_bounds,
        );

        // invoke transaction again
        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(
            starknet.pending_state.get_storage_at(blockifier_address, storage_key).unwrap(),
            Felt::from(25)
        );
    }

    #[test]
    fn invoke_transaction_v3_with_zero_gas_bounds_should_return_error() {
        let nonce = 0;
        let tx = test_invoke_transaction_v3(
            dummy_contract_address(),
            dummy_contract_address(),
            dummy_felt(),
            &[Felt::ZERO],
            nonce,
            resource_bounds_with_price_1(0, 0, 0),
        );

        match Starknet::default().add_invoke_transaction(tx) {
            Err(Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            )) => {}
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn invoke_transaction_should_return_an_error_if_same_nonce_supplied() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();

        let account_address = account.get_address();

        let nonce = 0;
        let tx = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            &[dummy_felt()],
            nonce,
            resource_bounds_with_price_1(0, 1000, 1e6 as u64),
        );

        let transaction_hash = starknet.add_invoke_transaction(tx.clone()).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);

        match starknet.add_invoke_transaction(tx) {
            Err(Error::TransactionValidationError(
                TransactionValidationError::InvalidTransactionNonce,
            )) => {}
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn nonce_should_be_incremented_if_invoke_reverted() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();

        let account_address = account.get_address().try_into().unwrap();
        let initial_nonce =
            starknet.pending_state.get_nonce_at(account_address).unwrap().0.try_into().unwrap();
        assert_eq!(initial_nonce, 0);

        let tx = test_invoke_transaction_v3(
            account_address.into(),
            contract_address,
            increase_balance_selector,
            &[dummy_felt()],
            initial_nonce,
            // if less, bounced back instead of accepted+reverted
            resource_bounds_with_price_1(0, 128, 480_000),
        );

        let transaction_hash = starknet.add_invoke_transaction(tx).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Reverted);

        let nonce_after_reverted = starknet.pending_state.get_nonce_at(account_address).unwrap();
        assert_eq!(nonce_after_reverted, Nonce(Felt::ONE));
    }

    /// Initialize starknet object with: erc20 contract, account contract and  simple contract that
    /// has a function increase_balance
    fn setup() -> (Starknet, Account, ContractAddress, Felt, ContractStorageKey) {
        let mut starknet = Starknet::default();

        // deploy erc20 contracts
        let eth_erc_20_contract =
            predeployed::tests::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        eth_erc_20_contract.deploy(&mut starknet.pending_state).unwrap();

        let strk_erc_20_contract =
            predeployed::tests::create_erc20_at_address(constants::STRK_ERC20_CONTRACT_ADDRESS)
                .unwrap();
        strk_erc_20_contract.deploy(&mut starknet.pending_state).unwrap();

        // deploy account contract
        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Balance::from(1000000000_u32),
            dummy_key_pair(),
            account_without_validations_class_hash,
            "Custom",
            ContractClass::Cairo0(account_without_validations_contract_class),
            eth_erc_20_contract.get_address(),
            strk_erc_20_contract.get_address(),
        )
        .unwrap();

        account.deploy(&mut starknet.pending_state).unwrap();

        // dummy contract
        let dummy_contract = dummy_cairo_0_contract_class();

        // check if increase_balance function is present in the contract class
        let increase_balance_selector = get_selector_from_name("increase_balance").unwrap();
        let sn_api_class: starknet_api::deprecated_contract_class::ContractClass =
            dummy_contract.clone().try_into().unwrap();
        sn_api_class
            .entry_points_by_type
            .get(&starknet_api::contract_class::EntryPointType::External)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == increase_balance_selector)
            .unwrap();

        let dummy_contract_address = ContractAddress::new(Felt::from(5)).unwrap();
        let dummy_contract_class_hash = dummy_contract.generate_hash().unwrap();
        let storage_key = get_storage_var_address("balance", &[]).unwrap();
        let contract_storage_key = ContractStorageKey::new(dummy_contract_address, storage_key);

        // declare dummy contract
        starknet
            .pending_state
            .declare_contract_class(dummy_contract_class_hash, None, dummy_contract.into())
            .unwrap();

        // deploy dummy contract
        starknet
            .pending_state
            .predeploy_contract(dummy_contract_address, dummy_contract_class_hash)
            .unwrap();
        // change storage of dummy contract

        starknet.block_context = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );
        starknet.next_block_gas = GasModification {
            gas_price_wei: nonzero!(1u128),
            data_gas_price_wei: nonzero!(1u128),
            l2_gas_price_wei: nonzero!(1u128),
            gas_price_fri: nonzero!(1u128),
            data_gas_price_fri: nonzero!(1u128),
            l2_gas_price_fri: nonzero!(1u128),
        };

        starknet.restart_pending_block().unwrap();

        (starknet, account, dummy_contract_address, increase_balance_selector, contract_storage_key)
    }
}
