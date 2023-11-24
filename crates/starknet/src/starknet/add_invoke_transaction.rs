use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use starknet_types::rpc::transactions::{InvokeTransaction, Transaction};

use super::Starknet;
use crate::error::{DevnetResult, Error};

pub fn add_invoke_transaction(
    starknet: &mut Starknet,
    broadcasted_invoke_transaction: BroadcastedInvokeTransaction,
) -> DevnetResult<TransactionHash> {
    if broadcasted_invoke_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "invoke transaction".into() });
    }

    let blockifier_invoke_transaction = broadcasted_invoke_transaction
        .create_blockifier_invoke_transaction(starknet.chain_id().to_felt(), false)?;
    let transaction_hash = blockifier_invoke_transaction.tx_hash.0.into();

    let invoke_transaction =
        broadcasted_invoke_transaction.create_invoke_transaction(transaction_hash);
    let transaction = Transaction::Invoke(InvokeTransaction::Version1(invoke_transaction));

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Invoke(
            blockifier_invoke_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, blockifier_execution_result)?;

    Ok(transaction_hash)
}

#[cfg(test)]
mod tests {
    use starknet_api::hash::StarkFelt;
    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut, StateChanger, StateExtractor};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::get_storage_var_address;
    use crate::utils::test_utils::{
        cairo_0_account_without_validations, dummy_contract_address, dummy_felt, get_bytes_from_u32,
    };

    fn test_invoke_transaction_v1(
        account_address: ContractAddress,
        contract_address: ContractAddress,
        function_selector: Felt,
        param: Felt,
        nonce: u128,
    ) -> BroadcastedInvokeTransaction {
        let calldata = vec![
            Felt::from(contract_address), // contract address
            function_selector,            // function selector
            Felt::from(1),                // calldata len
            param,                        // calldata
        ];

        BroadcastedInvokeTransaction::new(
            account_address,
            Fee(5000),
            &vec![],
            Felt::from(nonce),
            &calldata,
            Felt::from(1),
        )
    }

    #[test]
    fn invoke_transaction_successful_execution() {
        let (mut starknet, account_address, contract_address, increase_balance_selector, _) =
            setup();

        let invoke_transaction = test_invoke_transaction_v1(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(10),
            0,
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();

        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
    }

    #[test]
    fn invoke_transaction_successfully_changes_storage() {
        let (
            mut starknet,
            account_address,
            contract_address,
            increase_balance_selector,
            balance_var_storage_address,
        ) = setup();

        let invoke_transaction = test_invoke_transaction_v1(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(10),
            0,
        );

        // invoke transaction
        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check storage
        assert_eq!(
            starknet.state.get_storage(balance_var_storage_address).unwrap(),
            Felt::from(10)
        );

        let invoke_transaction = test_invoke_transaction_v1(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(15),
            1,
        );

        // invoke transaction again
        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(
            starknet.state.get_storage(balance_var_storage_address).unwrap(),
            Felt::from(25)
        );
    }

    #[test]
    fn invoke_transaction_with_max_fee_zero_should_return_error() {
        let invoke_transaction = BroadcastedInvokeTransaction::new(
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            &vec![],
            Felt::from(1),
        );

        let result = Starknet::default().add_invoke_transaction(invoke_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "invoke transaction: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn invoke_transaction_should_return_an_error_if_same_nonce_supplied() {
        let (mut starknet, account_address, contract_address, increase_balance_selector, _) =
            setup();

        let nonce = 0;
        let invoke_transaction = test_invoke_transaction_v1(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(10),
            nonce,
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction.clone()).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);

        match starknet.add_invoke_transaction(invoke_transaction).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InvalidTransactionNonce,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn nonce_should_be_incremented_if_invoke_reverted() {
        let (mut starknet, account_address, contract_address, increase_balance_selector, _) =
            setup();

        let initial_nonce = starknet.state.get_nonce(&account_address).unwrap();
        assert_eq!(initial_nonce, Felt::from(0));

        let calldata = vec![
            Felt::from(contract_address), // contract address
            increase_balance_selector,    // function selector
            Felt::from(1),                // calldata len
            Felt::from(10),               // calldata
        ];

        let insufficient_max_fee = 2482; // this is minimum fee (enough for passing validation), anything lower than that is bounced back
        let invoke_transaction = BroadcastedInvokeTransaction::new(
            account_address,
            Fee(insufficient_max_fee),
            &vec![],
            initial_nonce,
            &calldata,
            Felt::from(1),
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Reverted);

        let nonce_after_reverted = starknet.state.get_nonce(&account_address).unwrap();
        assert_eq!(nonce_after_reverted, Felt::from(1));
    }

    /// Initialize starknet object with: erc20 contract, account contract and  simple contract that
    /// has a function increase_balance
    fn setup() -> (Starknet, ContractAddress, ContractAddress, Felt, ContractStorageKey) {
        let mut starknet = Starknet::default();

        // deploy erc20 contract
        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        // deploy account contract
        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Felt::from(10000),
            dummy_felt(),
            dummy_felt(),
            account_without_validations_class_hash,
            ContractClass::Cairo0(account_without_validations_contract_class),
            erc_20_contract.get_address(),
        )
        .unwrap();

        account.deploy(&mut starknet.state).unwrap();
        account.set_initial_balance(&mut starknet.state).unwrap();

        // dummy contract
        let dummy_contract: Cairo0ContractClass = dummy_cairo_0_contract_class().into();
        let blockifier = blockifier::execution::contract_class::ContractClassV0::try_from(
            dummy_contract.clone(),
        )
        .unwrap();
        let increase_balance_selector =
            StarkFelt::new(get_selector_from_name("increase_balance").unwrap().to_bytes_be())
                .unwrap();

        // check if increase_balance function is present in the contract class
        blockifier
            .entry_points_by_type
            .get(&starknet_api::deprecated_contract_class::EntryPointType::External)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == increase_balance_selector)
            .unwrap();

        let mut address_bytes = get_bytes_from_u32(5);
        address_bytes.reverse();

        let dummy_contract_address =
            ContractAddress::new(Felt::new(address_bytes).unwrap()).unwrap();
        let dummy_contract_class_hash = dummy_contract.generate_hash().unwrap();
        let storage_key = get_storage_var_address("balance", &[]).unwrap();
        let contract_storage_key = ContractStorageKey::new(dummy_contract_address, storage_key);

        // declare dummy contract
        starknet
            .state
            .declare_contract_class(dummy_contract_class_hash, dummy_contract.into())
            .unwrap();

        // deploy dummy contract
        starknet.state.deploy_contract(dummy_contract_address, dummy_contract_class_hash).unwrap();
        // change storage of dummy contract
        // starknet.state.change_storage(contract_storage_key, Felt::from(0)).unwrap();

        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (
            starknet,
            account.get_address(),
            dummy_contract_address,
            Felt::from(increase_balance_selector),
            contract_storage_key,
        )
    }
}
