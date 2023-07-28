use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::starknet_api::transaction::Fee;

use super::json_rpc::error::ApiError;
use super::models::transaction::{
    DeclareTransaction, DeclareTransactionV0V1, DeclareTransactionV2, DeployAccountTransaction,
    InvokeTransactionV1, Transaction, TransactionType, TransactionWithType,
};
use super::models::{ContractAddressHex, FeltHex};

impl From<&Felt> for FeltHex {
    fn from(value: &Felt) -> Self {
        Self(*value)
    }
}

impl From<Felt> for FeltHex {
    fn from(value: Felt) -> Self {
        Self(value)
    }
}

impl From<ContractAddress> for ContractAddressHex {
    fn from(value: ContractAddress) -> Self {
        Self(value)
    }
}

impl From<&ContractAddress> for ContractAddressHex {
    fn from(value: &ContractAddress) -> Self {
        Self(*value)
    }
}

pub(crate) fn into_vec<'a, T, U>(value: &'a [T]) -> Vec<U>
where
    U: std::convert::From<&'a T>,
{
    value.iter().map(|x| U::from(x)).collect()
}

impl TryFrom<&starknet_core::transactions::Transaction> for TransactionWithType {
    type Error = ApiError;
    fn try_from(txn: &starknet_core::transactions::Transaction) -> Result<Self, Self::Error> {
        let transaction_with_type = match txn {
            starknet_core::transactions::Transaction::Declare(declare_v1) => {
                let declare_txn = DeclareTransactionV0V1 {
                    class_hash: declare_v1.class_hash().unwrap_or(&Felt::default()).into(),
                    sender_address: declare_v1.sender_address().into(),
                    nonce: txn.nonce().into(),
                    max_fee: Fee(txn.max_fee()),
                    version: txn.version().into(),
                    transaction_hash: txn.get_hash().unwrap_or_default().into(),
                    signature: into_vec(txn.signature()),
                };
                TransactionWithType {
                    r#type: TransactionType::Declare,
                    transaction: Transaction::Declare(DeclareTransaction::Version1(declare_txn)),
                }
            }
            starknet_core::transactions::Transaction::DeclareV2(declare_v2) => {
                let declare_txn = DeclareTransactionV2 {
                    class_hash: declare_v2.class_hash().unwrap_or(&Felt::default()).into(),
                    compiled_class_hash: declare_v2.compiled_class_hash().into(),
                    sender_address: declare_v2.sender_address().into(),
                    nonce: txn.nonce().into(),
                    max_fee: Fee(txn.max_fee()),
                    version: txn.version().into(),
                    transaction_hash: txn.get_hash().unwrap_or_default().into(),
                    signature: into_vec(txn.signature()),
                };

                TransactionWithType {
                    r#type: TransactionType::Declare,
                    transaction: Transaction::Declare(DeclareTransaction::Version2(declare_txn)),
                }
            }
            starknet_core::transactions::Transaction::DeployAccount(deploy_account) => {
                let deploy_account_txn = DeployAccountTransaction {
                    nonce: txn.nonce().into(),
                    max_fee: Fee(txn.max_fee()),
                    version: txn.version().into(),
                    transaction_hash: txn.get_hash().unwrap_or_default().into(),
                    signature: into_vec(txn.signature()),
                    class_hash: deploy_account
                        .class_hash()
                        .map_err(ApiError::StarknetDevnetError)?
                        .into(),
                    contract_address_salt: deploy_account.contract_address_salt().into(),
                    constructor_calldata: into_vec(&deploy_account.constructor_calldata()),
                };

                TransactionWithType {
                    r#type: TransactionType::DeployAccount,
                    transaction: Transaction::DeployAccount(deploy_account_txn),
                }
            }
            starknet_core::transactions::Transaction::Invoke(invoke_v1) => {
                let invoke_txn = InvokeTransactionV1 {
                    sender_address: invoke_v1
                        .sender_address()
                        .map_err(ApiError::StarknetDevnetError)?
                        .into(),
                    nonce: txn.nonce().into(),
                    max_fee: Fee(txn.max_fee()),
                    version: txn.version().into(),
                    transaction_hash: txn.get_hash().unwrap_or_default().into(),
                    signature: into_vec(txn.signature()),
                    calldata: into_vec(invoke_v1.calldata()),
                };

                TransactionWithType {
                    r#type: TransactionType::Invoke,
                    transaction: Transaction::Invoke(
                        crate::api::models::transaction::InvokeTransaction::Version1(invoke_txn),
                    ),
                }
            }
        };

        Ok(transaction_with_type)
    }
}
