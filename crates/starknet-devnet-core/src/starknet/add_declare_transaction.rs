use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use starknet_types::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
use starknet_types::rpc::transactions::declare_transaction_v2::DeclareTransactionV2;
use starknet_types::rpc::transactions::declare_transaction_v3::DeclareTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, DeclareTransaction, Transaction, TransactionWithHash,
};

use super::dump::DumpEvent;
use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;

pub fn add_declare_transaction_v3(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV3,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.is_max_fee_zero_value() {
        return Err(Error::MaxFeeZeroError { tx_type: "declare transaction v3".to_string() });
    }

    let blockifier_declare_transaction = broadcasted_declare_transaction
        .create_blockifier_declare(starknet.chain_id().to_felt(), false)?;

    let transaction_hash = blockifier_declare_transaction.tx_hash().0.into();
    let class_hash = blockifier_declare_transaction.class_hash().0.into();

    let transaction = TransactionWithHash::new(
        transaction_hash,
        Transaction::Declare(DeclareTransaction::V3(DeclareTransactionV3::new(
            &broadcasted_declare_transaction,
            class_hash,
        ))),
    );

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Declare(
            blockifier_declare_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(
        transaction,
        Some(broadcasted_declare_transaction.contract_class.clone().into()),
        blockifier_execution_result,
    )?;
    starknet.handle_dump_event(DumpEvent::AddDeclareTransaction(
        BroadcastedDeclareTransaction::V3(Box::new(broadcasted_declare_transaction)),
    ))?;

    Ok((transaction_hash, class_hash))
}

pub fn add_declare_transaction_v2(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV2,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "declare transaction v2".into() });
    }

    let blockifier_declare_transaction =
        broadcasted_declare_transaction.create_blockifier_declare(starknet.chain_id().to_felt())?;

    let transaction_hash = blockifier_declare_transaction.tx_hash().0.into();
    let class_hash = blockifier_declare_transaction.class_hash().0.into();

    let transaction = TransactionWithHash::new(
        transaction_hash,
        Transaction::Declare(DeclareTransaction::V2(DeclareTransactionV2::new(
            &broadcasted_declare_transaction,
            class_hash,
        ))),
    );

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Declare(
            blockifier_declare_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(
        transaction,
        Some(broadcasted_declare_transaction.contract_class.clone().into()),
        blockifier_execution_result,
    )?;
    starknet.handle_dump_event(DumpEvent::AddDeclareTransaction(
        BroadcastedDeclareTransaction::V2(Box::new(broadcasted_declare_transaction)),
    ))?;

    Ok((transaction_hash, class_hash))
}

pub fn add_declare_transaction_v1(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV1,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "declare transaction v1".into() });
    }

    let class_hash = broadcasted_declare_transaction.generate_class_hash()?;
    let transaction_hash = broadcasted_declare_transaction
        .calculate_transaction_hash(&starknet.config.chain_id.to_felt(), &class_hash)?;

    let declare_transaction =
        DeclareTransactionV0V1::new(&broadcasted_declare_transaction, class_hash);

    let transaction = TransactionWithHash::new(
        transaction_hash,
        Transaction::Declare(DeclareTransaction::V1(declare_transaction)),
    );

    let blockifier_declare_transaction =
        broadcasted_declare_transaction.create_blockifier_declare(class_hash, transaction_hash)?;

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Declare(
            blockifier_declare_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(
        transaction,
        Some(broadcasted_declare_transaction.contract_class.clone().into()),
        blockifier_execution_result,
    )?;
    starknet.handle_dump_event(DumpEvent::AddDeclareTransaction(
        BroadcastedDeclareTransaction::V1(Box::new(broadcasted_declare_transaction)),
    ))?;

    Ok((transaction_hash, class_hash))
}

#[cfg(test)]
mod tests {
    use blockifier::state::state_api::StateReader;
    use nonzero_ext::nonzero;
    use starknet_api::core::CompiledClassHash;
    use starknet_api::hash::StarkHash;
    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{
        BlockId, BlockTag, TransactionExecutionStatus, TransactionFinalityStatus,
    };
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0Json, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::state::Balance;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::starknet::predeployed::create_erc20_at_address;
    use crate::starknet::{predeployed, Starknet};
    use crate::state::CustomStateReader;
    use crate::traits::{Deployed, HashIdentified, HashIdentifiedMut};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        convert_broadcasted_declare_v2_to_v3, dummy_broadcasted_declare_transaction_v2,
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
    };

    fn broadcasted_declare_transaction_v1(
        sender_address: ContractAddress,
    ) -> BroadcastedDeclareTransactionV1 {
        let contract_class = dummy_cairo_0_contract_class();

        BroadcastedDeclareTransactionV1::new(
            sender_address,
            Fee(10000),
            &Vec::new(),
            Felt::from(0),
            &contract_class.into(),
            Felt::from(1),
        )
    }

    #[test]
    fn declare_transaction_v3_with_max_fee_zero_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV2::new(
            &dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            dummy_felt(),
        );

        let declare_transaction = convert_broadcasted_declare_v2_to_v3(declare_transaction);

        let result = Starknet::default().add_declare_transaction_v3(declare_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "declare transaction v3: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn declare_transaction_v2_with_max_fee_zero_should_return_an_error() {
        let declare_transaction_v2 = BroadcastedDeclareTransactionV2::new(
            &dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            dummy_felt(),
        );

        let result = Starknet::default().add_declare_transaction_v2(declare_transaction_v2);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "declare transaction v2: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);

        match starknet.add_declare_transaction_v2(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v3_transaction_successful_execution() {
        let (mut starknet, sender) = setup(Some(1e18 as u128));

        let declare_txn =
            convert_broadcasted_declare_v2_to_v3(dummy_broadcasted_declare_transaction_v2(&sender));

        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v3(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(
            class_hash,
            ContractClass::Cairo1(declare_txn.contract_class).generate_hash().unwrap()
        );
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        starknet.state.get_rpc_contract_class(&class_hash).unwrap();
    }

    #[test]
    fn add_declare_v2_transaction_successful_execution() {
        let (mut starknet, sender) = setup(Some(100000000));

        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v2(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        let generated_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();
        assert_eq!(class_hash, generated_hash);

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert_eq!(
            starknet.get_class(&BlockId::Tag(BlockTag::Latest), class_hash).unwrap(),
            declare_txn.contract_class.into()
        );
    }

    #[test]
    fn declare_v2_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup(Some(100000000));
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);
        let expected_class_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();
        let expected_compiled_class_hash = declare_txn.compiled_class_hash;

        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(expected_class_hash));
        assert_eq!(
            starknet.state.get_compiled_class_hash(expected_class_hash.into()).unwrap(),
            CompiledClassHash(StarkHash::ZERO)
        );
        assert!(starknet.get_class(&BlockId::Tag(BlockTag::Latest), expected_class_hash).is_err());

        let (tx_hash, retrieved_class_hash) =
            starknet.add_declare_transaction_v2(declare_txn).unwrap();

        let retrieved_txn = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(retrieved_class_hash, expected_class_hash);
        // check if txn is with status accepted
        assert_eq!(retrieved_txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(retrieved_txn.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(starknet.state.is_contract_declared(expected_class_hash));
        assert_eq!(
            starknet.state.get_compiled_class_hash(expected_class_hash.into()).unwrap(),
            expected_compiled_class_hash.into()
        );
    }

    #[test]
    fn declare_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV1::new(
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            &dummy_cairo_0_contract_class().into(),
            Felt::from(1),
        );

        let result = Starknet::default().add_declare_transaction_v1(declare_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "declare transaction v1: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_low_max_fee() {
        let (mut starknet, sender) = setup(Some(20000));

        let mut declare_txn = broadcasted_declare_transaction_v1(sender);
        declare_txn.common.max_fee = Fee(10);

        match starknet.add_declare_transaction_v1(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientMaxFee,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_not_enough_balance_on_account() {
        let (mut starknet, sender) = setup(Some(1));

        let declare_txn = broadcasted_declare_transaction_v1(sender);
        match starknet.add_declare_transaction_v1(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v1_transaction_successful_execution() {
        let (mut starknet, sender) = setup(None);

        let declare_txn = broadcasted_declare_transaction_v1(sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v1(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(class_hash, declare_txn.contract_class.generate_hash().unwrap());
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        // check if contract is successfully declared
        assert!(starknet.state.is_contract_declared(class_hash));
        // check if pending block is resetted
        assert!(starknet.pending_block().get_transactions().is_empty());
        // check if there is generated block
        assert_eq!(starknet.blocks.hash_to_block.len(), 1);
        // check if transaction is in generated block
        assert_eq!(
            *starknet
                .blocks
                .get_by_hash(starknet.blocks.last_block_hash.unwrap())
                .unwrap()
                .get_transactions()
                .first()
                .unwrap(),
            tx_hash
        );
    }

    #[test]
    fn declare_v1_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup(None);
        let declare_txn = broadcasted_declare_transaction_v1(sender);

        let expected_class_hash = declare_txn.contract_class.generate_hash().unwrap();
        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(expected_class_hash));

        let (tx_hash, class_hash) = starknet.add_declare_transaction_v1(declare_txn).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check if contract is declared
        assert!(starknet.state.is_contract_declared(class_hash));
    }

    /// Initializes starknet with 1 account - account without validations
    fn setup(acc_balance: Option<u128>) -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();

        let eth_erc_20_contract =
            predeployed::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        eth_erc_20_contract.deploy(&mut starknet.state).unwrap();

        let strk_erc20_contract = create_erc20_at_address(STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        strk_erc20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Balance::from(acc_balance.unwrap_or(10000)),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class.into(),
            eth_erc_20_contract.get_address(),
            strk_erc20_contract.get_address(),
        )
        .unwrap();

        acc.deploy(&mut starknet.state).unwrap();

        starknet.block_context = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
