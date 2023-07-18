use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{Felt, TransactionHash};
use starknet_types::traits::HashProducer;

use crate::error::{Error, Result};
use crate::traits::StateExtractor;
use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::transactions::{StarknetTransaction, Transaction};
use crate::Starknet;

impl Starknet {
    pub fn add_deploy_account_transaction(
        &mut self,
        deploy_account_transaction: DeployAccountTransaction,
    ) -> Result<(TransactionHash, ContractAddress)> {
        if !self
            .state
            .is_contract_declared(&Felt::new(*deploy_account_transaction.0.class_hash())?)?
        {
            return Err(Error::StateError(StateError::MissingClassHash()));
        }

        let state_before_txn = self.state.pending_state.clone();
        let transaction_hash = deploy_account_transaction.generate_hash()?;
        let address: ContractAddress =
            (deploy_account_transaction.0.contract_address().clone()).try_into()?;

        match deploy_account_transaction
            .0
            .execute(&mut self.state.pending_state, &self.block_context)
        {
            Ok(tx_info) => {
                self.handle_successful_transaction(
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

                self.transactions.insert(&transaction_hash, transaction_to_add);
                // Revert to previous pending state
                self.state.pending_state = state_before_txn;
            }
        }

        Ok((transaction_hash, address))
    }
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;

    use crate::constants::DEVNET_DEFAULT_CHAIN_ID;
    use crate::traits::{HashIdentifiedMut, StateChanger, StateExtractor};
    use crate::transactions::deploy_account_transaction::DeployAccountTransaction;
    use crate::utils::get_storage_var_address;
    use crate::utils::test_utils::setup;

    #[test]
    fn deploy_account_transaction_should_fail_due_to_low_balance() {
        let (mut starknet, addr, _) = setup(None);
        let account_class_hash = starknet.state.get_class_hash_at_contract_address(&addr).unwrap();
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
        let (mut starknet, addr, fee_token_address) = setup(None);
        let account_class_hash = starknet.state.get_class_hash_at_contract_address(&addr).unwrap();

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
}
