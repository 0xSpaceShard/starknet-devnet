use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{Felt, TransactionHash};
use starknet_types::traits::HashProducer;

use super::Starknet;
use crate::error::{Error, Result};
use crate::traits::StateExtractor;
use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::transactions::{StarknetTransaction, Transaction};

pub fn add_deploy_account_transaction(
    starknet: &mut Starknet,
    deploy_account_transaction: DeployAccountTransaction,
) -> Result<(TransactionHash, ContractAddress)> {
    if !starknet
        .state
        .is_contract_declared(&Felt::new(*deploy_account_transaction.0.class_hash())?)?
    {
        return Err(Error::StateError(StateError::MissingClassHash()));
    }

    let state_before_txn = starknet.state.pending_state.clone();
    let transaction_hash = deploy_account_transaction.generate_hash()?;
    let address: ContractAddress =
        (deploy_account_transaction.0.contract_address().clone()).try_into()?;

    match deploy_account_transaction
        .0
        .execute(&mut starknet.state.pending_state, &starknet.block_context)
    {
        Ok(tx_info) => {
            starknet.handle_successful_transaction(
                &transaction_hash,
                Transaction::DeployAccount(Box::new(deploy_account_transaction)),
                tx_info,
            )?;
        }
        Err(tx_err) => {
            let transaction_to_add = StarknetTransaction::create_rejected(
                Transaction::DeployAccount(Box::new(deploy_account_transaction)),
                tx_err,
            );

            starknet.transactions.insert(&transaction_hash, transaction_to_add);
            // Revert to previous pending state
            starknet.state.pending_state = state_before_txn;
        }
    }

    Ok((transaction_hash, address))
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::{ClassHash, Felt};
    use starknet_types::traits::HashProducer;

    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Accounted, HashIdentifiedMut, StateChanger, StateExtractor};
    use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
    use crate::utils::{get_storage_var_address, load_cairo_0_contract_class};

    #[test]
    fn deploy_account_transaction_should_fail_due_to_low_balance() {
        let (mut starknet, account_class_hash, _) = setup();

        let transaction = DeployAccountTransaction::new(
            vec![],
            2000,
            vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            DEVNET_DEFAULT_CHAIN_ID.to_felt().into(),
            Felt::from(0),
        )
        .unwrap();

        let (txn_hash, _) = starknet.add_deploy_account_transaction(transaction).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.status, TransactionStatus::Rejected);
    }

    #[test]
    fn deploy_account_transaction_successful_execution() {
        let (mut starknet, account_class_hash, fee_token_address) = setup();

        let transaction = DeployAccountTransaction::new(
            vec![],
            2000,
            vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            DEVNET_DEFAULT_CHAIN_ID.to_felt().into(),
            Felt::from(0),
        )
        .unwrap();

        let account_address =
            ContractAddress::try_from(transaction.0.contract_address().clone()).unwrap();
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()]).unwrap();
        let balance_storage_key =
            ContractStorageKey::new(fee_token_address, balance_storage_var_address);

        // change balance at address
        let account_balance_before_deployment = Felt::from(1000000);
        starknet
            .state
            .change_storage(balance_storage_key, account_balance_before_deployment)
            .unwrap();
        starknet.state.synchronize_states();

        // get accounts count before deployment
        let accounts_before_deployment = starknet.state.state.address_to_nonce.len();

        let (txn_hash, _) = starknet.add_deploy_account_transaction(transaction).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.status, TransactionStatus::AcceptedOnL2);
        assert_eq!(starknet.state.state.address_to_nonce.len(), accounts_before_deployment + 1);
        let account_balance_after_deployment =
            starknet.state.get_storage(balance_storage_key).unwrap();

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    /// Initializes starknet with 1 account - account without validations
    fn setup() -> (Starknet, ClassHash, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let contract_class = load_cairo_0_contract_class(account_json_path).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();
        starknet.state.declare_contract_class(class_hash, contract_class).unwrap();

        starknet.state.synchronize_states();
        starknet.block_context = Starknet::get_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        )
        .unwrap();

        starknet.restart_pending_block().unwrap();

        (starknet, class_hash, erc_20_contract.get_address())
    }
}
