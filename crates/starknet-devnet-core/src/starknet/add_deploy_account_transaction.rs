use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{Felt, TransactionHash};
use starknet_types::rpc::transactions::deploy_account_transaction_v1::DeployAccountTransactionV1;
use starknet_types::rpc::transactions::deploy_account_transaction_v3::DeployAccountTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeployAccountTransaction, DeployAccountTransaction, Transaction, TransactionWithHash,
};

use super::dump::DumpEvent;
use super::Starknet;
use crate::error::{DevnetResult, Error};
use crate::state::CustomStateReader;

pub fn add_deploy_account_transaction(
    starknet: &mut Starknet,
    broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransaction,
) -> DevnetResult<(TransactionHash, ContractAddress)> {
    let (
        transaction_hash,
        class_hash,
        address,
        blockifier_deploy_account_transaction,
        deploy_account_transaction,
    ) = match broadcasted_deploy_account_transaction {
        BroadcastedDeployAccountTransaction::V1(ref v1) => {
            if v1.common.max_fee.0 == 0 {
                return Err(Error::MaxFeeZeroError {
                    tx_type: "deploy account transaction".into(),
                });
            }

            let blockifier_deploy_account_transaction =
                v1.create_blockifier_deploy_account(starknet.chain_id().to_felt())?;

            let transaction_hash: Felt = blockifier_deploy_account_transaction.tx_hash.0.into();
            let address: ContractAddress =
                blockifier_deploy_account_transaction.contract_address.into();

            let deploy_account_transaction =
                Transaction::DeployAccount(DeployAccountTransaction::V1(Box::new(
                    DeployAccountTransactionV1::new(v1, address),
                )));

            (
                transaction_hash,
                v1.class_hash,
                address,
                blockifier_deploy_account_transaction,
                deploy_account_transaction,
            )
        }
        BroadcastedDeployAccountTransaction::V3(ref v3) => {
            if v3.common.is_max_fee_zero_value() {
                return Err(Error::MaxFeeZeroError {
                    tx_type: "deploy account transaction v3".into(),
                });
            }

            let blockifier_deploy_account_transaction =
                v3.create_blockifier_deploy_account(starknet.chain_id().to_felt())?;

            let transaction_hash: Felt = blockifier_deploy_account_transaction.tx_hash.0.into();
            let address: ContractAddress =
                blockifier_deploy_account_transaction.contract_address.into();

            let deploy_account_transaction =
                Transaction::DeployAccount(DeployAccountTransaction::V3(Box::new(
                    DeployAccountTransactionV3::new(v3, address),
                )));

            (
                transaction_hash,
                v3.class_hash,
                address,
                blockifier_deploy_account_transaction,
                deploy_account_transaction,
            )
        }
    };

    if blockifier_deploy_account_transaction.only_query {
        return Err(Error::UnsupportedAction {
            msg: "Only query transactions are not supported".to_string(),
        });
    }

    if !starknet.state.is_contract_declared(class_hash) {
        return Err(Error::StateError(crate::error::StateError::NoneClassHash(class_hash)));
    }

    let transaction = TransactionWithHash::new(transaction_hash, deploy_account_transaction);

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::DeployAccount(
            blockifier_deploy_account_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, None, blockifier_execution_result)?;
    starknet.handle_dump_event(DumpEvent::AddDeployAccountTransaction(
        broadcasted_deploy_account_transaction,
    ))?;

    Ok((transaction_hash, address))
}
#[cfg(test)]
mod tests {

    use blockifier::state::state_api::{State, StateReader};
    use nonzero_ext::nonzero;
    use starknet_api::hash::StarkFelt;
    use starknet_api::transaction::{Fee, Tip};
    use starknet_rs_core::types::{
        BlockId, BlockTag, TransactionExecutionStatus, TransactionFinalityStatus,
    };
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::Cairo0Json;
    use starknet_types::felt::{ClassHash, Felt};
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedDeployAccountTransaction, BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };
    use starknet_types::traits::HashProducer;

    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::error::Error;
    use crate::starknet::{predeployed, Starknet};
    use crate::state::CustomState;
    use crate::traits::{Deployed, HashIdentifiedMut};
    use crate::utils::get_storage_var_address;

    fn test_deploy_account_transaction_v3(
        class_hash: ClassHash,
        nonce: u128,
        l1_gas_amount: u64,
    ) -> BroadcastedDeployAccountTransactionV3 {
        BroadcastedDeployAccountTransactionV3 {
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
            contract_address_salt: 0.into(),
            constructor_calldata: vec![],
            class_hash,
        }
    }

    #[test]
    fn account_deploy_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let deploy_account_transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![0.into(), 1.into()],
            Fee(0),
            &vec![0.into(), 1.into()],
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        );

        let result = Starknet::default().add_deploy_account_transaction(
            BroadcastedDeployAccountTransaction::V1(deploy_account_transaction),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "deploy account transaction: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_v3_with_max_fee_zero_should_return_an_error() {
        let (mut starknet, account_class_hash, _, _) = setup();
        let deploy_account_transaction =
            test_deploy_account_transaction_v3(account_class_hash, 0, 0);

        let txn_err = starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(
                deploy_account_transaction,
            ))
            .unwrap_err();
        match txn_err {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "deploy account transaction v3: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn deploy_account_transaction_v1_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash, _, _) = setup();

        let fee_raw: u128 = 4000;
        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        match starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V1(transaction))
            .unwrap_err()
        {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn deploy_account_transaction_v3_should_return_an_error_due_to_not_enough_balance() {
        let (mut starknet, account_class_hash, _, _) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000);

        match starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(transaction))
            .unwrap_err()
        {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn deploy_account_transaction_v1_should_return_an_error_due_to_not_enough_fee() {
        let (mut starknet, account_class_hash, eth_fee_token_address, _) = setup();

        let fee_raw: u128 = 1;
        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(fee_raw),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );

        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let fee_token_address: starknet_api::core::ContractAddress =
            eth_fee_token_address.try_into().unwrap();
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let account_balance_before_deployment = StarkFelt::from_u128(1000000);
        starknet
            .state
            .set_storage_at(
                fee_token_address,
                balance_storage_var_address,
                account_balance_before_deployment,
            )
            .unwrap();

        match starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V1(transaction))
            .unwrap_err()
        {
            Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientMaxFee,
            ) => {}
            err => {
                panic!("Wrong error type: {:?}", err);
            }
        }
    }

    #[test]
    fn test_deploy_account_transaction_v3_successful_execution() {
        let (mut starknet, account_class_hash, _, strk_fee_token_address) = setup();
        let transaction = test_deploy_account_transaction_v3(account_class_hash, 0, 4000);

        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let fee_token_address: starknet_api::core::ContractAddress =
            strk_fee_token_address.try_into().unwrap();
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let account_balance_before_deployment = StarkFelt::from_u128(1000000);
        starknet
            .state
            .set_storage_at(
                fee_token_address,
                balance_storage_var_address,
                account_balance_before_deployment,
            )
            .unwrap();

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

        let account_balance_after_deployment =
            starknet.state.get_storage_at(fee_token_address, balance_storage_var_address).unwrap();

        assert!(account_balance_before_deployment > account_balance_after_deployment);
    }

    #[test]
    fn deploy_account_transaction_v1_successful_execution() {
        let (mut starknet, account_class_hash, eth_fee_token_address, _) = setup();

        let transaction = BroadcastedDeployAccountTransactionV1::new(
            &vec![],
            Fee(4000),
            &vec![],
            Felt::from(0),
            account_class_hash,
            Felt::from(13),
            Felt::from(1),
        );
        let blockifier_transaction = transaction
            .create_blockifier_deploy_account(DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap();

        // change balance at address
        let account_address = ContractAddress::from(blockifier_transaction.contract_address);
        let fee_token_address: starknet_api::core::ContractAddress =
            eth_fee_token_address.try_into().unwrap();
        let balance_storage_var_address =
            get_storage_var_address("ERC20_balances", &[account_address.into()])
                .unwrap()
                .try_into()
                .unwrap();

        let account_balance_before_deployment = StarkFelt::from_u128(1000000);
        starknet
            .state
            .set_storage_at(
                fee_token_address,
                balance_storage_var_address,
                account_balance_before_deployment,
            )
            .unwrap();

        let (txn_hash, _) = starknet
            .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V1(transaction))
            .unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert_eq!(
            starknet.get_class_hash_at(&BlockId::Tag(BlockTag::Latest), account_address).unwrap(),
            account_class_hash,
        );
    }

    /// Initializes starknet with erc20 contract, 1 declared contract class. Gas price is set to 1
    fn setup() -> (Starknet, ClassHash, ContractAddress, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let erc_20_contract =
            predeployed::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let strk_erc20_contract =
            predeployed::create_erc20_at_address(STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        strk_erc20_contract.deploy(&mut starknet.state).unwrap();

        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();

        starknet.state.declare_contract_class(class_hash, contract_class.into()).unwrap();
        starknet.block_context = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, class_hash, erc_20_contract.get_address(), strk_erc20_contract.get_address())
    }
}
