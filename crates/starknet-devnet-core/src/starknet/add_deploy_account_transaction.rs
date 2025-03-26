use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::deploy_account_transaction_v3::DeployAccountTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeployAccountTransaction, DeployAccountTransaction, Transaction, TransactionWithHash,
};

use super::Starknet;
use crate::error::{DevnetResult, Error, TransactionValidationError};
use crate::state::CustomStateReader;

pub fn add_deploy_account_transaction(
    starknet: &mut Starknet,
    broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransaction,
) -> DevnetResult<(TransactionHash, ContractAddress)> {
    if !broadcasted_deploy_account_transaction.is_max_fee_valid() {
        return Err(TransactionValidationError::InsufficientResourcesForValidate.into());
    }

    if broadcasted_deploy_account_transaction.is_only_query() {
        return Err(Error::UnsupportedAction {
            msg: "only-query transactions are not supported".to_string(),
        });
    }

    let executable_deploy_account_tx = broadcasted_deploy_account_transaction
        .create_sn_api_deploy_account(&starknet.chain_id().to_felt())?;

    let address = executable_deploy_account_tx.contract_address.into();

    let (class_hash, deploy_account_transaction) = match broadcasted_deploy_account_transaction {
        BroadcastedDeployAccountTransaction::V3(ref v3) => {
            let deploy_account_transaction =
                Transaction::DeployAccount(DeployAccountTransaction::V3(Box::new(
                    DeployAccountTransactionV3::new(v3, address),
                )));

            (v3.class_hash, deploy_account_transaction)
        }
    };

    if !starknet.pending_state.is_contract_declared(class_hash) {
        return Err(Error::StateError(crate::error::StateError::NoneClassHash(class_hash)));
    }
    let transaction_hash = executable_deploy_account_tx.tx_hash.0;
    let transaction = TransactionWithHash::new(transaction_hash, deploy_account_transaction);

    let execution_info = blockifier::transaction::account_transaction::AccountTransaction {
        tx: starknet_api::executable_transaction::AccountTransaction::DeployAccount(
            executable_deploy_account_tx,
        ),
        execution_flags: ExecutionFlags { only_query: false, charge_fee: true, validate: true },
    }
    .execute(&mut starknet.pending_state.state, &starknet.block_context)?;

    starknet.handle_accepted_transaction(transaction, execution_info)?;

    Ok((transaction_hash, address))
}
#[cfg(test)]
mod tests {

    use blockifier::state::state_api::{State, StateReader};
    use nonzero_ext::nonzero;
    use starknet_api::transaction::fields::{Fee, Tip};
    use starknet_rs_core::types::{
        BlockId, BlockTag, Felt, TransactionExecutionStatus, TransactionFinalityStatus,
    };
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::ClassHash;
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedDeployAccountTransaction, BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };
    use starknet_types::traits::HashProducer;

    use crate::constants::{
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER, ETH_ERC20_CONTRACT_ADDRESS,
        STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::{Error, TransactionValidationError};
    use crate::starknet::{Starknet, predeployed};
    use crate::state::CustomState;
    use crate::traits::{Deployed, HashIdentifiedMut};
    use crate::utils::get_storage_var_address;
    use crate::utils::test_utils::cairo_0_account_without_validations;

    const GAS_PRICE: u128 = 1_u128;

    // TODO add test for all three gas bounds

    fn test_deploy_account_transaction_v3(
        class_hash: ClassHash,
        nonce: u128,
        l1_gas_amount: u64,
        l1_data_gas_amount: u64,
        l2_gas_amount: u64,
    ) -> BroadcastedDeployAccountTransactionV3 {
        BroadcastedDeployAccountTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce: Felt::from(nonce),
                resource_bounds: ResourceBoundsWrapper::new(
                    l1_gas_amount,
                    GAS_PRICE,
                    l1_data_gas_amount,
                    GAS_PRICE,
                    l2_gas_amount,
                    GAS_PRICE,
                ),
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            contract_address_salt: 0.into(),
            constructor_calldata: vec![],
            class_hash,
        }
    }

    #[test]
    fn account_deploy_transaction_v3_with_query_version_should_return_an_error() {
        let mut deploy_account_transaction =
            test_deploy_account_transaction_v3(Felt::default(), 0, 10, 0, 0);
        deploy_account_transaction.common.version = Felt::THREE + QUERY_VERSION_OFFSET;

        let result = Starknet::default().add_deploy_account_transaction(
            BroadcastedDeployAccountTransaction::V3(deploy_account_transaction),
        );

        match result {
            Err(crate::error::Error::UnsupportedAction { msg }) => {
                assert_eq!(msg, "only-query transactions are not supported")
            }
            other => panic!("Unexpected result: {other:?}"),
        };
    }

    #[test]
    fn deploy_account_transaction_v3_with_max_fee_zero_should_return_an_error() {
        let (mut starknet, account_class_hash) = setup();
        let deploy_account_transaction =
            test_deploy_account_transaction_v3(account_class_hash, 0, 0, 0, 0);

        let txn_err = starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(
                deploy_account_transaction,
            ))
            .unwrap_err();
        match txn_err {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_v3_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000, 0, 0);

        match starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(transaction))
            .unwrap_err()
        {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => panic!("Wrong error type: {err:?}"),
        }
    }

    fn get_strk_balance(starknet: &Starknet, address: ContractAddress) -> Felt {
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let erc20_address =
            starknet.block_context.chain_info().fee_token_addresses.strk_fee_token_address;
        starknet.pending_state.get_storage_at(erc20_address, balance_storage_var_address).unwrap()
    }

    fn set_strk_balance(starknet: &mut Starknet, address: ContractAddress, amount: Felt) {
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let erc20_address =
            starknet.block_context.chain_info().fee_token_addresses.strk_fee_token_address;
        starknet
            .pending_state
            .set_storage_at(erc20_address, balance_storage_var_address, amount)
            .unwrap();
    }

    #[test]
    fn deploy_account_transaction_v1_should_return_an_error_due_to_not_enough_fee() {
        let (mut starknet, account_class_hash) = setup();

        let fee_raw: u128 = 1;
        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::ZERO,
            account_class_hash,
            Felt::from(13),
            Felt::ONE,
        );

        let executable_tx = BroadcastedDeployAccountTransaction::V1(transaction.clone())
            .create_sn_api_deploy_account(&DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(executable_tx.contract_address);
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let account_balance_before_deployment = Felt::from(1_000_000);
        starknet
            .pending_state
            .set_storage_at(
                starknet.block_context.chain_info().fee_token_addresses.eth_fee_token_address,
                balance_storage_var_address,
                account_balance_before_deployment,
            )
            .unwrap();

        match starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V1(transaction))
            .unwrap_err()
        {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            err => panic!("Wrong error type: {err:?}"),
        }
    }

    #[test]
    fn test_deploy_account_transaction_v3_successful_execution() {
        let (mut starknet, account_class_hash) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000, 0, 0);

        let executable_tx = BroadcastedDeployAccountTransaction::V3(transaction.clone())
            .create_sn_api_deploy_account(&DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap();

        let account_address = ContractAddress::from(executable_tx.contract_address);
        let account_balance_before_deployment = Felt::from(1_000_000);
        set_strk_balance(&mut starknet, account_address, account_balance_before_deployment);

        let (txn_hash, _) = starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(transaction))
            .unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert_eq!(
            starknet.get_class_hash_at(&BlockId::Tag(BlockTag::Latest), account_address).unwrap(),
            account_class_hash,
        );

        let account_balance_after_deployment = get_strk_balance(&starknet, account_address);

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    /// Initializes starknet with erc20 contracts, 1 declared contract class. Gas price is set to 1
    fn setup() -> (Starknet, ClassHash) {
        let mut starknet = Starknet::default();

        for erc20_address in [ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS] {
            let erc20_contract =
                predeployed::tests::create_erc20_at_address(erc20_address).unwrap();
            erc20_contract.deploy(&mut starknet.pending_state).unwrap();
        }

        let contract_class = cairo_0_account_without_validations(); // TODO use cairo1
        let class_hash = contract_class.generate_hash().unwrap();

        starknet
            .pending_state
            .declare_contract_class(class_hash, None, contract_class.into())
            .unwrap();
        starknet.block_context = Starknet::init_block_context(
            nonzero!(GAS_PRICE),
            nonzero!(GAS_PRICE),
            nonzero!(GAS_PRICE),
            nonzero!(GAS_PRICE),
            nonzero!(GAS_PRICE),
            nonzero!(GAS_PRICE),
            ETH_ERC20_CONTRACT_ADDRESS,
            STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, class_hash)
    }
}
