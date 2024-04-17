use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::invoke_transaction_v1::InvokeTransactionV1;
use starknet_types::rpc::transactions::invoke_transaction_v3::InvokeTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedInvokeTransaction, InvokeTransaction, Transaction, TransactionWithHash,
};

use super::dump::DumpEvent;
use super::Starknet;
use crate::error::{DevnetResult, Error};

pub fn add_invoke_transaction(
    starknet: &mut Starknet,
    broadcasted_invoke_transaction: BroadcastedInvokeTransaction,
) -> DevnetResult<TransactionHash> {
    if broadcasted_invoke_transaction.is_max_fee_zero_value() {
        return Err(Error::MaxFeeZeroError { tx_type: broadcasted_invoke_transaction.to_string() });
    }

    let blockifier_invoke_transaction = broadcasted_invoke_transaction
        .create_blockifier_invoke_transaction(&starknet.chain_id().to_felt())?;

    if blockifier_invoke_transaction.only_query {
        return Err(Error::UnsupportedAction {
            msg: "query-only transactions are not supported".to_string(),
        });
    }

    let transaction_hash = blockifier_invoke_transaction.tx_hash.0.into();

    let invoke_transaction = match broadcasted_invoke_transaction {
        BroadcastedInvokeTransaction::V1(ref v1) => {
            Transaction::Invoke(InvokeTransaction::V1(InvokeTransactionV1::new(v1)))
        }
        BroadcastedInvokeTransaction::V3(ref v3) => {
            Transaction::Invoke(InvokeTransaction::V3(InvokeTransactionV3::new(v3)))
        }
    };

    let validate = if Starknet::is_account_impersonated(
        &mut starknet.state,
        &starknet.cheats,
        &ContractAddress::from(blockifier_invoke_transaction.sender_address()),
    )? {
        false
    } else {
        true
    };

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Invoke(
            blockifier_invoke_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, validate);

    let transaction = TransactionWithHash::new(transaction_hash, invoke_transaction);

    starknet.handle_transaction_result(transaction, None, blockifier_execution_result)?;
    starknet.handle_dump_event(DumpEvent::AddInvokeTransaction(broadcasted_invoke_transaction))?;

    Ok(transaction_hash)
}
#[cfg(test)]
mod tests {

    use core::panic;

    use blockifier::state::state_api::StateReader;
    use nonzero_ext::nonzero;
    use starknet_api::core::Nonce;
    use starknet_api::hash::StarkFelt;
    use starknet_api::transaction::{Fee, Tip};
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_ff::FieldElement;
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;
    use starknet_types::rpc::state::Balance;
    use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedInvokeTransaction, BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };
    use starknet_types::traits::HashProducer;

    use crate::account::{Account, FeeToken};
    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        ETH_ERC20_CONTRACT_ADDRESS,
    };
    use crate::starknet::{predeployed, Starknet};
    use crate::state::CustomState;
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut};
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

        BroadcastedInvokeTransaction::V1(BroadcastedInvokeTransactionV1::new(
            account_address,
            Fee(5000),
            &vec![],
            Felt::from(nonce),
            &calldata,
            Felt::from(1),
        ))
    }

    fn test_invoke_transaction_v3(
        account_address: ContractAddress,
        contract_address: ContractAddress,
        function_selector: Felt,
        param: Felt,
        nonce: u128,
        l1_gas_amount: u64,
    ) -> BroadcastedInvokeTransaction {
        let calldata = vec![
            Felt::from(contract_address), // contract address
            function_selector,            // function selector
            Felt::from(1),                // calldata len
            param,                        // calldata
        ];

        BroadcastedInvokeTransaction::V3(BroadcastedInvokeTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::from(3),
                signature: vec![],
                nonce: Felt::from(nonce),
                resource_bounds: ResourceBoundsWrapper::new(l1_gas_amount, 1, 0, 0),
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            sender_address: account_address,
            calldata,
            account_deployment_data: vec![],
        })
    }

    #[test]
    fn invoke_transaction_v3_with_only_query_version_should_return_an_error() {
        let mut invoke_transaction = test_invoke_transaction_v3(
            dummy_contract_address(),
            dummy_contract_address(),
            dummy_felt(),
            Felt::from(10),
            0,
            1,
        );
        match invoke_transaction {
            BroadcastedInvokeTransaction::V3(ref mut v3) => {
                v3.common.version = (FieldElement::from(3u8) + QUERY_VERSION_OFFSET).into();
            }
            _ => {
                panic!("Wrong transaction type");
            }
        }

        let txn_err = Starknet::default().add_invoke_transaction(invoke_transaction).unwrap_err();
        match txn_err {
            crate::error::Error::UnsupportedAction { msg } => {
                assert_eq!(msg, "query-only transactions are not supported".to_string());
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn invoke_transaction_v3_should_fail_due_to_no_fee_provided() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();
        let account_address = account.get_address();

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(10),
            0,
            0,
        );

        let invoke_v3_txn_error = starknet
            .add_invoke_transaction(invoke_transaction)
            .expect_err("Expected MaxFeeZeroError");

        match invoke_v3_txn_error {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(
                    err.to_string(),
                    "Invoke transaction V3: max_fee cannot be zero".to_string()
                );
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn invoke_transaction_v3_successful_execution() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();
        let account_address = account.get_address();
        let initial_balance = account.get_balance(&mut starknet.state, FeeToken::STRK).unwrap();

        let invoke_transaction = test_invoke_transaction_v3(
            account_address,
            contract_address,
            increase_balance_selector,
            Felt::from(10),
            0,
            account
                .get_balance(&mut starknet.state, crate::account::FeeToken::STRK)
                .unwrap()
                .to_string()
                .parse::<u64>()
                .unwrap(),
        );

        let transaction_hash = starknet.add_invoke_transaction(invoke_transaction).unwrap();

        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(
            account.get_balance(&mut starknet.state, FeeToken::STRK).unwrap() < initial_balance
        );
    }

    #[test]
    fn invoke_transaction_successful_execution() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();

        let account_address = account.get_address();

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
            account,
            contract_address,
            increase_balance_selector,
            balance_var_storage_address,
        ) = setup();
        let blockifier_address: starknet_api::core::ContractAddress =
            contract_address.try_into().unwrap();
        let storage_key = (*balance_var_storage_address.get_storage_key()).try_into().unwrap();

        let account_address = account.get_address();

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
            starknet.state.get_storage_at(blockifier_address, storage_key).unwrap(),
            Felt::from(10).into()
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
            starknet.state.get_storage_at(blockifier_address, storage_key).unwrap(),
            StarkFelt::from_u128(25)
        );
    }

    #[test]
    fn invoke_transaction_with_max_fee_zero_should_return_error() {
        let invoke_transaction = BroadcastedInvokeTransactionV1::new(
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            &vec![],
            Felt::from(1),
        );

        let result = Starknet::default()
            .add_invoke_transaction(BroadcastedInvokeTransaction::V1(invoke_transaction));

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "Invoke transaction V1: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn invoke_transaction_should_return_an_error_if_same_nonce_supplied() {
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();

        let account_address = account.get_address();

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
        let (mut starknet, account, contract_address, increase_balance_selector, _) = setup();

        let account_address: starknet_api::core::ContractAddress =
            account.get_address().try_into().unwrap();
        let initial_nonce = starknet.state.get_nonce_at(account_address).unwrap();
        assert_eq!(initial_nonce, Nonce(StarkFelt::ZERO));

        let calldata = vec![
            Felt::from(contract_address), // contract address
            increase_balance_selector,    // function selector
            Felt::from(1),                // calldata len
            Felt::from(10),               // calldata
        ];

        let insufficient_max_fee = 137; // this is minimum fee (enough for passing validation), anything lower than that is bounced back
        let invoke_transaction = BroadcastedInvokeTransactionV1::new(
            account_address.into(),
            Fee(insufficient_max_fee),
            &vec![],
            initial_nonce.into(),
            &calldata,
            Felt::from(1),
        );

        let transaction_hash = starknet
            .add_invoke_transaction(BroadcastedInvokeTransaction::V1(invoke_transaction))
            .unwrap();
        let transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();
        assert_eq!(transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(transaction.execution_result.status(), TransactionExecutionStatus::Reverted);

        let nonce_after_reverted = starknet.state.get_nonce_at(account_address).unwrap();
        assert_eq!(nonce_after_reverted, Nonce(StarkFelt::ONE));
    }

    /// Initialize starknet object with: erc20 contract, account contract and  simple contract that
    /// has a function increase_balance
    fn setup() -> (Starknet, Account, ContractAddress, Felt, ContractStorageKey) {
        let mut starknet = Starknet::default();

        // deploy erc20 contracts
        let eth_erc_20_contract =
            predeployed::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        eth_erc_20_contract.deploy(&mut starknet.state).unwrap();

        let strk_erc_20_contract =
            predeployed::create_erc20_at_address(constants::STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        strk_erc_20_contract.deploy(&mut starknet.state).unwrap();

        // deploy account contract
        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Balance::from(10000_u32),
            dummy_felt(),
            dummy_felt(),
            account_without_validations_class_hash,
            ContractClass::Cairo0(account_without_validations_contract_class),
            eth_erc_20_contract.get_address(),
            strk_erc_20_contract.get_address(),
        )
        .unwrap();

        account.deploy(&mut starknet.state).unwrap();

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
        starknet
            .state
            .predeploy_contract(dummy_contract_address, dummy_contract_class_hash)
            .unwrap();
        // change storage of dummy contract

        starknet.block_context = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );

        starknet.restart_pending_block().unwrap();

        (
            starknet,
            account,
            dummy_contract_address,
            Felt::from(increase_balance_selector),
            contract_storage_key,
        )
    }
}
