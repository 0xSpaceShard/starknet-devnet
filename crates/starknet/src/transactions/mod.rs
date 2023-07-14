pub mod declare_transaction;
pub(crate) mod declare_transaction_v2;

use std::collections::HashMap;

use starknet_api::block::BlockNumber;
use starknet_in_rust::execution::TransactionExecutionInfo;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_rs_core::types::TransactionStatus;
use starknet_types::felt::{BlockHash, TransactionHash};

use self::declare_transaction::DeclareTransactionV1;
use self::declare_transaction_v2::DeclareTransactionV2;
use crate::traits::HashIdentifiedMut;

#[derive(Default)]
pub struct StarknetTransactions(HashMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
    }

    pub fn get(&self, transaction_hash: &TransactionHash) -> Option<&StarknetTransaction>
    {
        self.0.get(transaction_hash)
    }
}

impl HashIdentifiedMut for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash_mut(&mut self, hash: &Self::Hash) -> Option<&mut StarknetTransaction> {
        self.0.get_mut(hash)
    }
}

#[allow(unused)]
#[derive(Clone)]
pub struct StarknetTransaction {
    pub(crate) status: TransactionStatus,
    inner: Transaction,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    execution_error: Option<TransactionError>,
}

// impl Clone for TransactionError {
//     fn clone(&self) -> Self {
//         match self {
//             Self::MissingNonce => Self::MissingNonce,
//             Self::InvalidMaxFee => Self::InvalidMaxFee,
//             Self::InvalidNonce => Self::InvalidNonce,
//             Self::InvalidSignature => Self::InvalidSignature,
//             Self::InvokeFunctionNonZeroMissingNonce => Self::InvokeFunctionNonZeroMissingNonce,
//             Self::InvokeFunctionZeroHasNonce => Self::InvokeFunctionZeroHasNonce,
//             Self::InvalidTransactionNonce(arg0, arg1) => Self::InvalidTransactionNonce(arg0.clone(), arg1.clone()),
//             Self::ActualFeeExceedsMaxFee(arg0, arg1) => Self::ActualFeeExceedsMaxFee(arg0.clone(), arg1.clone()),
//             Self::FeeTransferError(arg0) => Self::FeeTransferError(arg0.clone()),
//             Self::FeeError(arg0) => Self::FeeError(arg0.clone()),
//             Self::ResourcesError => Self::ResourcesError,
//             Self::ResourcesCalculation => Self::ResourcesCalculation,
//             Self::ContractAddress(arg0) => Self::ContractAddress(arg0.clone()),
//             Self::Syscall(arg0) => Self::Syscall(arg0.clone()),
//             Self::State(arg0) => Self::State(arg0.clone()),
//             Self::UnauthorizedActionOnValidate => Self::UnauthorizedActionOnValidate,
//             Self::ClassAlreadyDeclared(arg0) => Self::ClassAlreadyDeclared(arg0.clone()),
//             Self::NotARelocatableValue => Self::NotARelocatableValue,
//             Self::UnexpectedHolesInEventOrder => Self::UnexpectedHolesInEventOrder,
//             Self::UnexpectedHolesL2toL1Messages => Self::UnexpectedHolesL2toL1Messages,
//             Self::CallTypeIsNotDelegate => Self::CallTypeIsNotDelegate,
//             Self::AttempToUseNoneCodeAddress => Self::AttempToUseNoneCodeAddress,
//             Self::FailToReadClassHash => Self::FailToReadClassHash,
//             Self::MissingCompiledClass => Self::MissingCompiledClass,
//             Self::NotDeployedContract(arg0) => Self::NotDeployedContract(arg0.clone()),
//             Self::NonUniqueEntryPoint => Self::NonUniqueEntryPoint,
//             Self::EntryPointNotFound => Self::EntryPointNotFound,
//             Self::OsContextPtrNotEqual => Self::OsContextPtrNotEqual,
//             Self::EmptyOsContext => Self::EmptyOsContext,
//             Self::IllegalOsPtrOffset => Self::IllegalOsPtrOffset,
//             Self::InvalidPtrFetch => Self::InvalidPtrFetch,
//             Self::InvalidSegBasePtrOffset(arg0) => Self::InvalidSegBasePtrOffset(arg0.clone()),
//             Self::InvalidSegmentSize => Self::InvalidSegmentSize,
//             Self::InvalidStopPointer(arg0, arg1) => Self::InvalidStopPointer(arg0.clone(), arg1.clone()),
//             Self::InvalidEntryPoints => Self::InvalidEntryPoints,
//             Self::NotAFelt => Self::NotAFelt,
//             Self::OutOfBound => Self::OutOfBound,
//             Self::InvalidContractCall => Self::InvalidContractCall,
//             Self::InvalidSenderAddress => Self::InvalidSenderAddress,
//             Self::TraceException(arg0) => Self::TraceException(arg0.clone()),
//             Self::MemoryException(arg0) => Self::MemoryException(arg0.clone()),
//             Self::MissingInitialFp => Self::MissingInitialFp,
//             Self::InvalidTxContext => Self::InvalidTxContext,
//             Self::SierraCompileError(arg0) => Self::SierraCompileError(arg0.clone()),
//             Self::InvalidBuiltinContractClass(arg0) => Self::InvalidBuiltinContractClass(arg0.clone()),
//             Self::NotEqualClassHash => Self::NotEqualClassHash,
//             Self::Vm(arg0) => Self::Vm(arg0.clone()),
//             Self::CairoRunner(arg0) => Self::CairoRunner(arg0.clone()),
//             Self::Runner(arg0) => Self::Runner(arg0.clone()),
//             Self::NoneTransactionType(arg0, arg1) => Self::NoneTransactionType(arg0.clone(), arg1.clone()),
//             Self::MathError(arg0) => Self::MathError(arg0.clone()),
//             Self::ProgramError(arg0) => Self::ProgramError(arg0.clone()),
//             Self::EmptyConstructorCalldata => Self::EmptyConstructorCalldata,
//             Self::InvalidBlockNumber => Self::InvalidBlockNumber,
//             Self::InvalidBlockTimestamp => Self::InvalidBlockTimestamp,
//             Self::CustomError(arg0) => Self::CustomError(arg0.clone()),
//             Self::CallInfoIsNone => Self::CallInfoIsNone,
//             Self::UnsupportedVersion(arg0) => Self::UnsupportedVersion(arg0.clone()),
//         }
//     }
// }

impl StarknetTransaction {
    pub fn create_rejected(transaction: Transaction, execution_error: TransactionError) -> Self {
        Self {
            status: TransactionStatus::Rejected,
            inner: transaction,
            execution_info: None,
            execution_error: Some(execution_error),
            block_hash: None,
            block_number: None,
        }
    }

    pub fn create_successful(
        transaction: Transaction,
        execution_info: TransactionExecutionInfo,
    ) -> Self {
        Self {
            status: TransactionStatus::Pending,
            inner: transaction,
            execution_info: Some(execution_info),
            execution_error: None,
            block_hash: None,
            block_number: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Transaction {
    Declare(DeclareTransactionV1),
    DeclareV2(DeclareTransactionV2),
}

impl Transaction {
    pub(crate) fn get_hash(&self) -> Option<TransactionHash> {
        match self {
            Transaction::Declare(tx) => tx.transaction_hash,
            Transaction::DeclareV2(tx) => tx.transaction_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::TransactionExecutionInfo;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::traits::HashProducer;

    use super::{StarknetTransaction, StarknetTransactions, Transaction};
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::dummy_declare_transaction_v1;

    #[test]
    fn get_transaction_by_hash() {
        let hash = dummy_declare_transaction_v1().generate_hash().unwrap();
        let sn_tx = StarknetTransaction::create_successful(
            Transaction::Declare(dummy_declare_transaction_v1()),
            TransactionExecutionInfo::default(),
        );
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            &hash,
            StarknetTransaction::create_successful(
                Transaction::Declare(dummy_declare_transaction_v1()),
                TransactionExecutionInfo::default(),
            ),
        );

        let extracted_tran = sn_txs.get_by_hash_mut(&hash).unwrap();

        assert_eq!(sn_tx.block_hash, extracted_tran.block_hash);
        assert_eq!(sn_tx.block_number, extracted_tran.block_number);
        assert!(sn_tx.inner == extracted_tran.inner);
        assert_eq!(sn_tx.status, extracted_tran.status);
        assert_eq!(sn_tx.execution_error.is_some(), extracted_tran.execution_error.is_some());
        assert_eq!(sn_tx.execution_info.is_some(), extracted_tran.execution_info.is_some());
    }

    #[test]
    fn check_correct_rejected_transaction_creation() {
        check_correct_transaction_properties(
            Transaction::Declare(dummy_declare_transaction_v1()),
            false,
        );
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        check_correct_transaction_properties(
            Transaction::Declare(dummy_declare_transaction_v1()),
            true,
        );
    }

    fn check_correct_transaction_properties(tran: Transaction, is_success: bool) {
        let sn_tran = if is_success {
            StarknetTransaction::create_successful(
                tran.clone(),
                TransactionExecutionInfo::default(),
            )
        } else {
            StarknetTransaction::create_rejected(
                tran.clone(),
                starknet_in_rust::transaction::error::TransactionError::AttempToUseNoneCodeAddress,
            )
        };

        if is_success {
            assert!(sn_tran.status == TransactionStatus::Pending);
        } else {
            assert!(sn_tran.status == TransactionStatus::Rejected);
        }

        assert_eq!(sn_tran.execution_info.is_some(), is_success);
        assert_eq!(sn_tran.execution_error.is_none(), is_success);
        assert!(sn_tran.block_hash.is_none());
        assert!(sn_tran.block_number.is_none());
        assert!(sn_tran.inner == tran);
    }
}
