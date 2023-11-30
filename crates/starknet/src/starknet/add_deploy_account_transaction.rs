use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::Transaction;

use super::Starknet;
use crate::error::{DevnetResult, Error};
use crate::traits::StateExtractor;

pub fn add_deploy_account_transaction(
    starknet: &mut Starknet,
    broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransaction,
) -> DevnetResult<(TransactionHash, ContractAddress)> {
    if broadcasted_deploy_account_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "deploy account transaction".into() });
    }

    if !starknet.state.is_contract_declared(&broadcasted_deploy_account_transaction.class_hash) {
        return Err(Error::StateError(crate::error::StateError::NoneClassHash(
            broadcasted_deploy_account_transaction.class_hash,
        )));
    }

    let blockifier_deploy_account_transaction = broadcasted_deploy_account_transaction
        .create_blockifier_deploy_account(starknet.chain_id().to_felt(), false)?;

    let transaction_hash = blockifier_deploy_account_transaction.tx_hash.0.into();
    let address: ContractAddress = blockifier_deploy_account_transaction.contract_address.into();
    let deploy_account_transaction = broadcasted_deploy_account_transaction
        .compile_deploy_account_transaction(&transaction_hash, address);

    let transaction = Transaction::DeployAccount(deploy_account_transaction);

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::DeployAccount(
            blockifier_deploy_account_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, blockifier_execution_result)?;

    Ok((transaction_hash, address))
}

#[cfg(test)]
mod tests {
    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::Cairo0Json;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::{ClassHash, Felt};
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
    use starknet_types::traits::HashProducer;

    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::error::Error;
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Deployed, HashIdentifiedMut, StateChanger, StateExtractor};
    use crate::utils::get_storage_var_address;

    #[test]
    fn account_deploy_transaction_with_max_fee_zero_should_return_an_error() {
        let deploy_account_transaction = BroadcastedDeployAccountTransaction::new(
            &vec![0.into(), 1.into()],
            Fee(0),
            &vec![0.into(), 1.into()],
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        );

        let result = Starknet::default().add_deploy_account_transaction(deploy_account_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "deploy account transaction: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash, _) = setup();

        let fee_raw: u128 = 4000;
        let transaction = BroadcastedDeployAccountTransaction::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        match starknet.add_deploy_account_transaction(transaction).unwrap_err() {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn deploy_account_transaction_should_return_an_error_due_to_not_enough_fee() {
        let (mut starknet, account_class_hash, fee_token_address) = setup();

        let fee_raw: u128 = 2000;
        let transaction = BroadcastedDeployAccountTransaction::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt(), false)
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(fee_token_address, balance_storage_var_address);

        starknet.state.change_storage(balance_storage_key, Felt::from(fee_raw)).unwrap();
        starknet.state.clear_dirty_state();

        match starknet.add_deploy_account_transaction(transaction).unwrap_err() {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientMaxFee,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    fn get_accounts_count(starknet: &Starknet) -> usize {
        starknet.state.state.state.address_to_class_hash.len()
    }

    #[test]
    fn deploy_account_transaction_successful_execution() {
        let (mut starknet, account_class_hash, fee_token_address) = setup();

        let transaction = BroadcastedDeployAccountTransaction::new(
            &vec![],
            Fee(4000),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );
        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt(), false)
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(fee_token_address, balance_storage_var_address);

        let account_balance_before_deployment = Felt::from(1000000);
        starknet
            .state
            .change_storage(balance_storage_key, account_balance_before_deployment)
            .unwrap();
        starknet.state.clear_dirty_state();

        // get accounts count before deployment
        let accounts_before_deployment = get_accounts_count(&starknet);

        let (txn_hash, _) = starknet.add_deploy_account_transaction(transaction).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert_eq!(get_accounts_count(&starknet), accounts_before_deployment + 1);
        let account_balance_after_deployment =
            starknet.state.get_storage(balance_storage_key).unwrap();

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    /// Initializes starknet with erc20 contract, 1 declared contract class. Gas price is set to 1
    fn setup() -> (Starknet, ClassHash, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();

        starknet.state.declare_contract_class(class_hash, contract_class.into()).unwrap();
        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, class_hash, erc_20_contract.get_address())
    }
}
