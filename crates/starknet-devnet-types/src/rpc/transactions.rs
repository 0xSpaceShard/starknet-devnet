use std::sync::Arc;

use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::objects::TransactionExecutionInfo;
use broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use declare_transaction_v0v1::DeclareTransactionV0V1;
use declare_transaction_v2::DeclareTransactionV2;
use deploy_transaction::DeployTransaction;
use invoke_transaction_v1::InvokeTransactionV1;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_api::block::{BlockNumber, GasPrice};
use starknet_api::contract_class::{ClassInfo, EntryPointType};
use starknet_api::core::calculate_contract_address;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::transaction::fields::{AllResourceBounds, Fee, Tip};
use starknet_api::transaction::{signed_tx_version, TransactionHasher, TransactionOptions};
use starknet_rs_core::types::{
    BlockId, ExecutionResult, Felt, ResourceBounds, ResourceBoundsMapping,
    TransactionFinalityStatus,
};
use starknet_rs_core::utils::parse_cairo_short_string;

use self::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use self::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use self::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use self::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
use self::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use self::declare_transaction_v3::DeclareTransactionV3;
use self::deploy_account_transaction_v1::DeployAccountTransactionV1;
use self::deploy_account_transaction_v3::DeployAccountTransactionV3;
use self::invoke_transaction_v3::InvokeTransactionV3;
use self::l1_handler_transaction::L1HandlerTransaction;
use super::estimate_message_fee::FeeEstimateWrapper;
use super::messaging::{MessageToL1, OrderedMessageToL1};
use super::state::ThinStateDiff;
use super::transaction_receipt::{
    ComputationResources, ExecutionResources, FeeInUnits, TransactionReceipt,
};
use crate::constants::QUERY_VERSION_OFFSET;
use crate::contract_address::ContractAddress;
use crate::contract_class::{compute_sierra_class_hash, ContractClass};
use crate::emitted_event::{Event, OrderedEvent};
use crate::error::{ConversionError, DevnetResult};
use crate::felt::{
    BlockHash, Calldata, EntryPointSelector, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transaction_receipt::{CommonTransactionReceipt, MaybePendingProperties};
use crate::{impl_wrapper_deserialize, impl_wrapper_serialize};

pub mod broadcasted_declare_transaction_v1;
pub mod broadcasted_declare_transaction_v2;
pub mod broadcasted_declare_transaction_v3;
pub mod broadcasted_deploy_account_transaction_v1;
pub mod broadcasted_deploy_account_transaction_v3;
pub mod broadcasted_invoke_transaction_v1;
pub mod broadcasted_invoke_transaction_v3;

pub mod declare_transaction_v0v1;
pub mod declare_transaction_v2;
pub mod declare_transaction_v3;
pub mod deploy_account_transaction_v1;
pub mod deploy_account_transaction_v3;
pub mod deploy_transaction;
pub mod invoke_transaction_v1;
pub mod invoke_transaction_v3;

pub mod l1_handler_transaction;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize))]
#[serde(untagged)]
pub enum Transactions {
    Hashes(Vec<TransactionHash>),
    Full(Vec<TransactionWithHash>),
    FullWithReceipts(Vec<TransactionWithReceipt>),
}

#[derive(Debug, Copy, Clone, Serialize, Default)]
#[cfg_attr(feature = "testing", derive(Deserialize))]
pub enum TransactionType {
    #[serde(rename(deserialize = "DECLARE", serialize = "DECLARE"))]
    Declare,
    #[serde(rename(deserialize = "DEPLOY", serialize = "DEPLOY"))]
    Deploy,
    #[serde(rename(deserialize = "DEPLOY_ACCOUNT", serialize = "DEPLOY_ACCOUNT"))]
    DeployAccount,
    #[serde(rename(deserialize = "INVOKE", serialize = "INVOKE"))]
    #[default]
    Invoke,
    #[serde(rename(deserialize = "L1_HANDLER", serialize = "L1_HANDLER"))]
    L1Handler,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "testing", derive(Deserialize, PartialEq, Eq), serde(deny_unknown_fields))]
pub enum Transaction {
    Declare(DeclareTransaction),
    DeployAccount(DeployAccountTransaction),
    Deploy(DeployTransaction),
    Invoke(InvokeTransaction),
    L1Handler(L1HandlerTransaction),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize, PartialEq, Eq))]
pub struct TransactionWithHash {
    transaction_hash: TransactionHash,
    #[serde(flatten)]
    pub transaction: Transaction,
}

impl TransactionWithHash {
    pub fn new(transaction_hash: TransactionHash, transaction: Transaction) -> Self {
        Self { transaction_hash, transaction }
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }

    pub fn get_type(&self) -> TransactionType {
        match self.transaction {
            Transaction::Declare(_) => TransactionType::Declare,
            Transaction::DeployAccount(_) => TransactionType::DeployAccount,
            Transaction::Deploy(_) => TransactionType::Deploy,
            Transaction::Invoke(_) => TransactionType::Invoke,
            Transaction::L1Handler(_) => TransactionType::L1Handler,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_common_receipt(
        &self,
        transaction_events: &[Event],
        transaction_messages_sent: &[MessageToL1],
        block_hash: Option<&BlockHash>,
        block_number: Option<BlockNumber>,
        execution_result: &ExecutionResult,
        finality_status: TransactionFinalityStatus,
        actual_fee: FeeInUnits,
        execution_info: &TransactionExecutionInfo,
    ) -> CommonTransactionReceipt {
        let r#type = self.get_type();
        let execution_resources = ExecutionResources::from(execution_info);
        let maybe_pending_properties =
            MaybePendingProperties { block_number, block_hash: block_hash.cloned() };

        CommonTransactionReceipt {
            r#type,
            transaction_hash: *self.get_transaction_hash(),
            actual_fee,
            messages_sent: transaction_messages_sent.to_vec(),
            events: transaction_events.to_vec(),
            execution_status: execution_result.clone(),
            finality_status,
            maybe_pending_properties,
            execution_resources,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize), serde(deny_unknown_fields))]
pub struct TransactionWithReceipt {
    pub receipt: TransactionReceipt,
    pub transaction: Transaction,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize, PartialEq, Eq))]
#[serde(untagged)]
pub enum DeclareTransaction {
    V1(DeclareTransactionV0V1),
    V2(DeclareTransactionV2),
    V3(DeclareTransactionV3),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize, PartialEq, Eq))]
#[serde(untagged)]
pub enum InvokeTransaction {
    V1(InvokeTransactionV1),
    V3(InvokeTransactionV3),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize, PartialEq, Eq))]
#[serde(untagged)]
pub enum DeployAccountTransaction {
    V1(Box<DeployAccountTransactionV1>),
    V3(Box<DeployAccountTransactionV3>),
}

impl DeployAccountTransaction {
    pub fn get_contract_address(&self) -> &ContractAddress {
        match self {
            DeployAccountTransaction::V1(tx) => tx.get_contract_address(),
            DeployAccountTransaction::V3(tx) => tx.get_contract_address(),
        }
    }
}

pub fn deserialize_paid_fee_on_l1<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let err_msg = format!("paid_fee_on_l1: expected 0x-prefixed hex string, got: {buf}");
    if !buf.starts_with("0x") {
        return Err(serde::de::Error::custom(err_msg));
    }
    u128::from_str_radix(&buf[2..], 16).map_err(|_| serde::de::Error::custom(err_msg))
}

fn serialize_paid_fee_on_l1<S>(paid_fee_on_l1: &u128, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{paid_fee_on_l1:#x}"))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventFilter {
    pub from_block: Option<BlockId>,
    pub to_block: Option<BlockId>,
    pub address: Option<ContractAddress>,
    pub keys: Option<Vec<Vec<Felt>>>,
    pub continuation_token: Option<String>,
    pub chunk_size: usize,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
pub struct EventsChunk {
    pub events: Vec<crate::emitted_event::EmittedEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq), serde(deny_unknown_fields))]
pub struct FunctionCall {
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommon {
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
}

fn is_only_query_common(version: &Felt) -> bool {
    version >= &QUERY_VERSION_OFFSET
}

impl BroadcastedTransactionCommon {
    pub fn is_max_fee_zero_value(&self) -> bool {
        self.max_fee.0 == 0
    }

    pub fn is_only_query(&self) -> bool {
        is_only_query_common(&self.version)
    }
}

fn felt_to_sn_api_chain_id(f: &Felt) -> DevnetResult<starknet_api::core::ChainId> {
    Ok(starknet_api::core::ChainId::Other(
        parse_cairo_short_string(f).map_err(|e| ConversionError::OutOfRangeError(e.to_string()))?,
    ))
}

/// Common fields for all transaction type of version 3
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommonV3 {
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    pub resource_bounds: ResourceBoundsWrapper,
    pub tip: Tip,
    pub paymaster_data: Vec<Felt>,
    pub nonce_data_availability_mode: DataAvailabilityMode,
    pub fee_data_availability_mode: DataAvailabilityMode,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "testing", derive(PartialEq, Eq))]
pub struct ResourceBoundsWrapper {
    inner: ResourceBoundsMapping,
}

impl_wrapper_serialize!(ResourceBoundsWrapper);
impl_wrapper_deserialize!(ResourceBoundsWrapper, ResourceBoundsMapping);

impl ResourceBoundsWrapper {
    pub fn new(
        l1_gas_max_amount: u64,
        l1_gas_max_price_per_unit: u128,
        l2_gas_max_amount: u64,
        l2_gas_max_price_per_unit: u128,
    ) -> Self {
        ResourceBoundsWrapper {
            inner: ResourceBoundsMapping {
                l1_gas: ResourceBounds {
                    max_amount: l1_gas_max_amount,
                    max_price_per_unit: l1_gas_max_price_per_unit,
                },
                l2_gas: ResourceBounds {
                    max_amount: l2_gas_max_amount,
                    max_price_per_unit: l2_gas_max_price_per_unit,
                },
            },
        }
    }
}

fn convert_resource_bounds_from_starknet_rs_to_starknet_api(
    bounds: ResourceBounds,
) -> starknet_api::transaction::fields::ResourceBounds {
    starknet_api::transaction::fields::ResourceBounds {
        max_amount: starknet_api::execution_resources::GasAmount(bounds.max_amount),
        max_price_per_unit: GasPrice(bounds.max_price_per_unit),
    }
}

impl From<&ResourceBoundsWrapper> for starknet_api::transaction::fields::ValidResourceBounds {
    fn from(value: &ResourceBoundsWrapper) -> Self {
        if value.inner.l2_gas.max_amount == 0 {
            starknet_api::transaction::fields::ValidResourceBounds::L1Gas(
                convert_resource_bounds_from_starknet_rs_to_starknet_api(
                    value.inner.l1_gas.clone(),
                ),
            )
        } else {
            starknet_api::transaction::fields::ValidResourceBounds::AllResources(
                AllResourceBounds {
                    l1_gas: convert_resource_bounds_from_starknet_rs_to_starknet_api(
                        value.inner.l1_gas.clone(),
                    ),
                    l2_gas: convert_resource_bounds_from_starknet_rs_to_starknet_api(
                        value.inner.l2_gas.clone(),
                    ),
                    l1_data_gas: convert_resource_bounds_from_starknet_rs_to_starknet_api(
                        // TODO gas
                        ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                    ),
                },
            )
        }
    }
}

impl BroadcastedTransactionCommonV3 {
    /// Checks if total accumulated fee of resource_bounds for l1 is equal to 0 or for l2 is not
    /// zero
    pub fn is_l1_gas_zero_or_l2_gas_not_zero(&self) -> bool {
        let l2_is_not_zero = (self.resource_bounds.inner.l2_gas.max_amount as u128)
            * self.resource_bounds.inner.l2_gas.max_price_per_unit
            > 0;
        let l1_is_zero = (self.resource_bounds.inner.l1_gas.max_amount as u128)
            * self.resource_bounds.inner.l1_gas.max_price_per_unit
            == 0;

        l1_is_zero || l2_is_not_zero
    }

    pub fn is_only_query(&self) -> bool {
        is_only_query_common(&self.version)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BroadcastedTransaction {
    Invoke(BroadcastedInvokeTransaction),
    Declare(BroadcastedDeclareTransaction),
    DeployAccount(BroadcastedDeployAccountTransaction),
}

impl BroadcastedTransaction {
    pub fn to_blockifier_account_transaction(
        &self,
        chain_id: &Felt,
        execution_flags: ExecutionFlags,
    ) -> DevnetResult<blockifier::transaction::account_transaction::AccountTransaction> {
        let sn_api_tx = self.to_sn_api_account_transaction(chain_id)?;
        Ok(blockifier::transaction::account_transaction::AccountTransaction {
            tx: sn_api_tx,
            execution_flags,
        })
    }

    pub fn to_sn_api_account_transaction(
        &self,
        chain_id: &Felt,
    ) -> DevnetResult<starknet_api::executable_transaction::AccountTransaction> {
        let sn_api_tx = match self {
            BroadcastedTransaction::Invoke(invoke_txn) => {
                starknet_api::executable_transaction::AccountTransaction::Invoke(
                    invoke_txn.create_sn_api_invoke(chain_id)?,
                )
            }
            BroadcastedTransaction::Declare(declare_txn) => {
                starknet_api::executable_transaction::AccountTransaction::Declare(
                    declare_txn.create_sn_api_declare(chain_id)?,
                )
            }
            BroadcastedTransaction::DeployAccount(deploy_account_txn) => {
                starknet_api::executable_transaction::AccountTransaction::DeployAccount(
                    deploy_account_txn.create_sn_api_deploy_account(chain_id)?,
                )
            }
        };

        Ok(sn_api_tx)
    }

    pub fn get_type(&self) -> TransactionType {
        match self {
            BroadcastedTransaction::Invoke(_) => TransactionType::Invoke,
            BroadcastedTransaction::Declare(_) => TransactionType::Declare,
            BroadcastedTransaction::DeployAccount(_) => TransactionType::DeployAccount,
        }
    }

    pub fn is_max_fee_zero_value(&self) -> bool {
        match self {
            BroadcastedTransaction::Invoke(broadcasted_invoke_transaction) => {
                broadcasted_invoke_transaction.is_max_fee_zero_value()
            }
            BroadcastedTransaction::Declare(broadcasted_declare_transaction) => {
                broadcasted_declare_transaction.is_max_fee_zero_value()
            }
            BroadcastedTransaction::DeployAccount(broadcasted_deploy_account_transaction) => {
                broadcasted_deploy_account_transaction.is_max_fee_zero_value()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
    V3(Box<BroadcastedDeclareTransactionV3>),
}

impl BroadcastedDeclareTransaction {
    pub fn is_max_fee_zero_value(&self) -> bool {
        match self {
            BroadcastedDeclareTransaction::V1(v1) => v1.common.is_max_fee_zero_value(),
            BroadcastedDeclareTransaction::V2(v2) => v2.common.is_max_fee_zero_value(),
            BroadcastedDeclareTransaction::V3(v3) => v3.common.is_l1_gas_zero_or_l2_gas_not_zero(),
        }
    }

    pub fn is_only_query(&self) -> bool {
        match self {
            BroadcastedDeclareTransaction::V1(tx) => tx.common.is_only_query(),
            BroadcastedDeclareTransaction::V2(tx) => tx.common.is_only_query(),
            BroadcastedDeclareTransaction::V3(tx) => tx.common.is_only_query(),
        }
    }

    /// Creates a blockifier declare transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    pub fn create_sn_api_declare(
        &self,
        chain_id: &Felt,
    ) -> DevnetResult<starknet_api::executable_transaction::DeclareTransaction> {
        let sn_api_chain_id = felt_to_sn_api_chain_id(chain_id)?;

        let (sn_api_transaction, tx_hash, class_info) = match self {
            BroadcastedDeclareTransaction::V1(v1) => {
                let class_hash = v1.generate_class_hash()?;

                let sn_api_declare = starknet_api::transaction::DeclareTransaction::V1(
                    starknet_api::transaction::DeclareTransactionV0V1 {
                        class_hash: starknet_api::core::ClassHash(class_hash),
                        sender_address: v1.sender_address.try_into()?,
                        nonce: starknet_api::core::Nonce(v1.common.nonce),
                        max_fee: v1.common.max_fee,
                        signature: starknet_api::transaction::fields::TransactionSignature(
                            v1.common.signature.clone(),
                        ),
                    },
                );

                let class_info: ClassInfo =
                    ContractClass::Cairo0(v1.contract_class.clone()).try_into()?;

                let tx_hash = v1.calculate_transaction_hash(chain_id, &class_hash)?;
                let sn_api_tx_hash = starknet_api::transaction::TransactionHash(tx_hash);

                (sn_api_declare, sn_api_tx_hash, class_info)
            }
            BroadcastedDeclareTransaction::V2(v2) => {
                let sierra_class_hash: Felt = compute_sierra_class_hash(&v2.contract_class)?;

                let sn_api_declare = starknet_api::transaction::DeclareTransaction::V2(
                    starknet_api::transaction::DeclareTransactionV2 {
                        max_fee: v2.common.max_fee,
                        signature: starknet_api::transaction::fields::TransactionSignature(
                            v2.common.signature.clone(),
                        ),
                        nonce: starknet_api::core::Nonce(v2.common.nonce),
                        class_hash: starknet_api::core::ClassHash(sierra_class_hash),
                        compiled_class_hash: starknet_api::core::CompiledClassHash(
                            v2.compiled_class_hash,
                        ),
                        sender_address: v2.sender_address.try_into()?,
                    },
                );

                let class_info: ClassInfo =
                    ContractClass::Cairo1(v2.contract_class.clone()).try_into()?;

                let tx_version: starknet_api::transaction::TransactionVersion = signed_tx_version(
                    &sn_api_declare.version(),
                    &TransactionOptions { only_query: self.is_only_query() },
                );

                let tx_hash =
                    sn_api_declare.calculate_transaction_hash(&sn_api_chain_id, &tx_version)?;

                (sn_api_declare, tx_hash, class_info)
            }
            BroadcastedDeclareTransaction::V3(v3) => {
                let sierra_class_hash = compute_sierra_class_hash(&v3.contract_class)?;

                let sn_api_declare = starknet_api::transaction::DeclareTransaction::V3(
                    starknet_api::transaction::DeclareTransactionV3 {
                        resource_bounds: (&v3.common.resource_bounds).into(),
                        tip: v3.common.tip,
                        signature: starknet_api::transaction::fields::TransactionSignature(
                            v3.common.signature.clone(),
                        ),
                        nonce: starknet_api::core::Nonce(v3.common.nonce),
                        class_hash: starknet_api::core::ClassHash(sierra_class_hash),
                        compiled_class_hash: starknet_api::core::CompiledClassHash(
                            v3.compiled_class_hash,
                        ),
                        sender_address: v3.sender_address.try_into()?,
                        nonce_data_availability_mode: v3.common.nonce_data_availability_mode,
                        fee_data_availability_mode: v3.common.fee_data_availability_mode,
                        paymaster_data: starknet_api::transaction::fields::PaymasterData(
                            v3.common.paymaster_data.clone(),
                        ),
                        account_deployment_data:
                            starknet_api::transaction::fields::AccountDeploymentData(
                                v3.account_deployment_data.clone(),
                            ),
                    },
                );

                let class_info: ClassInfo =
                    ContractClass::Cairo1(v3.contract_class.clone()).try_into()?;

                let tx_version: starknet_api::transaction::TransactionVersion = signed_tx_version(
                    &sn_api_declare.version(),
                    &TransactionOptions { only_query: self.is_only_query() },
                );

                let tx_hash =
                    sn_api_declare.calculate_transaction_hash(&sn_api_chain_id, &tx_version)?;

                (sn_api_declare, tx_hash, class_info)
            }
        };

        Ok(starknet_api::executable_transaction::DeclareTransaction {
            tx: sn_api_transaction,
            tx_hash,
            class_info,
        })
    }
}

#[derive(Debug, Clone)]
pub enum BroadcastedDeployAccountTransaction {
    V1(BroadcastedDeployAccountTransactionV1),
    V3(BroadcastedDeployAccountTransactionV3),
}

impl BroadcastedDeployAccountTransaction {
    pub fn is_max_fee_zero_value(&self) -> bool {
        match self {
            BroadcastedDeployAccountTransaction::V1(v1) => v1.common.is_max_fee_zero_value(),
            BroadcastedDeployAccountTransaction::V3(v3) => {
                v3.common.is_l1_gas_zero_or_l2_gas_not_zero()
            }
        }
    }

    pub fn is_only_query(&self) -> bool {
        match self {
            BroadcastedDeployAccountTransaction::V1(tx) => tx.common.is_only_query(),
            BroadcastedDeployAccountTransaction::V3(tx) => tx.common.is_only_query(),
        }
    }

    /// Creates a blockifier deploy account transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    pub fn create_sn_api_deploy_account(
        &self,
        chain_id: &Felt,
    ) -> DevnetResult<starknet_api::executable_transaction::DeployAccountTransaction> {
        let sn_api_transaction = match self {
            BroadcastedDeployAccountTransaction::V1(v1) => {
                let sn_api_transaction = starknet_api::transaction::DeployAccountTransactionV1 {
                    max_fee: v1.common.max_fee,
                    signature: starknet_api::transaction::fields::TransactionSignature(
                        v1.common.signature.clone(),
                    ),
                    nonce: starknet_api::core::Nonce(v1.common.nonce),
                    class_hash: starknet_api::core::ClassHash(v1.class_hash),
                    contract_address_salt: starknet_api::transaction::fields::ContractAddressSalt(
                        v1.contract_address_salt,
                    ),
                    constructor_calldata: starknet_api::transaction::fields::Calldata(Arc::new(
                        v1.constructor_calldata.clone(),
                    )),
                };

                starknet_api::transaction::DeployAccountTransaction::V1(sn_api_transaction)
            }
            BroadcastedDeployAccountTransaction::V3(v3) => {
                let sn_api_transaction = starknet_api::transaction::DeployAccountTransactionV3 {
                    resource_bounds: (&v3.common.resource_bounds).into(),
                    tip: v3.common.tip,
                    signature: starknet_api::transaction::fields::TransactionSignature(
                        v3.common.signature.clone(),
                    ),
                    nonce: starknet_api::core::Nonce(v3.common.nonce),
                    class_hash: starknet_api::core::ClassHash(v3.class_hash),
                    nonce_data_availability_mode: v3.common.nonce_data_availability_mode,
                    fee_data_availability_mode: v3.common.fee_data_availability_mode,
                    paymaster_data: starknet_api::transaction::fields::PaymasterData(
                        v3.common.paymaster_data.clone(),
                    ),
                    contract_address_salt: starknet_api::transaction::fields::ContractAddressSalt(
                        v3.contract_address_salt,
                    ),
                    constructor_calldata: starknet_api::transaction::fields::Calldata(Arc::new(
                        v3.constructor_calldata.clone(),
                    )),
                };

                starknet_api::transaction::DeployAccountTransaction::V3(sn_api_transaction)
            }
        };

        let chain_id = felt_to_sn_api_chain_id(chain_id)?;
        let tx_version: starknet_api::transaction::TransactionVersion = signed_tx_version(
            &sn_api_transaction.version(),
            &TransactionOptions { only_query: self.is_only_query() },
        );
        let tx_hash = sn_api_transaction.calculate_transaction_hash(&chain_id, &tx_version)?;

        // copied from starknet_api::executable_transaction::DeployAccountTransaction::create(
        let contract_address = calculate_contract_address(
            sn_api_transaction.contract_address_salt(),
            sn_api_transaction.class_hash(),
            &sn_api_transaction.constructor_calldata(),
            starknet_api::core::ContractAddress::default(),
        )?;

        Ok(starknet_api::executable_transaction::DeployAccountTransaction {
            tx: sn_api_transaction,
            tx_hash,
            contract_address,
        })
    }
}

#[derive(Debug, Clone)]
pub enum BroadcastedInvokeTransaction {
    V1(BroadcastedInvokeTransactionV1),
    V3(BroadcastedInvokeTransactionV3),
}

impl BroadcastedInvokeTransaction {
    pub fn is_max_fee_zero_value(&self) -> bool {
        match self {
            BroadcastedInvokeTransaction::V1(v1) => v1.common.is_max_fee_zero_value(),
            BroadcastedInvokeTransaction::V3(v3) => v3.common.is_l1_gas_zero_or_l2_gas_not_zero(),
        }
    }

    pub fn is_only_query(&self) -> bool {
        match self {
            BroadcastedInvokeTransaction::V1(tx) => tx.common.is_only_query(),
            BroadcastedInvokeTransaction::V3(tx) => tx.common.is_only_query(),
        }
    }

    /// Creates a blockifier invoke transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    pub fn create_sn_api_invoke(
        &self,
        chain_id: &Felt,
    ) -> DevnetResult<starknet_api::executable_transaction::InvokeTransaction> {
        let sn_api_transaction = match self {
            BroadcastedInvokeTransaction::V1(v1) => {
                let sn_api_transaction = starknet_api::transaction::InvokeTransactionV1 {
                    max_fee: v1.common.max_fee,
                    signature: starknet_api::transaction::fields::TransactionSignature(
                        v1.common.signature.clone(),
                    ),
                    nonce: starknet_api::core::Nonce(v1.common.nonce),
                    sender_address: v1.sender_address.try_into()?,
                    calldata: starknet_api::transaction::fields::Calldata(Arc::new(
                        v1.calldata.clone(),
                    )),
                };

                starknet_api::transaction::InvokeTransaction::V1(sn_api_transaction)
            }
            BroadcastedInvokeTransaction::V3(v3) => {
                let sn_api_transaction = starknet_api::transaction::InvokeTransactionV3 {
                    resource_bounds: (&v3.common.resource_bounds).into(),
                    tip: v3.common.tip,
                    signature: starknet_api::transaction::fields::TransactionSignature(
                        v3.common.signature.clone(),
                    ),
                    nonce: starknet_api::core::Nonce(v3.common.nonce),
                    sender_address: v3.sender_address.try_into()?,
                    calldata: starknet_api::transaction::fields::Calldata(Arc::new(
                        v3.calldata.clone(),
                    )),
                    nonce_data_availability_mode: v3.common.nonce_data_availability_mode,
                    fee_data_availability_mode: v3.common.fee_data_availability_mode,
                    paymaster_data: starknet_api::transaction::fields::PaymasterData(
                        v3.common.paymaster_data.clone(),
                    ),
                    account_deployment_data:
                        starknet_api::transaction::fields::AccountDeploymentData(
                            v3.account_deployment_data.clone(),
                        ),
                };

                starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction)
            }
        };

        let chain_id = felt_to_sn_api_chain_id(chain_id)?;
        let tx_version: starknet_api::transaction::TransactionVersion = signed_tx_version(
            &sn_api_transaction.version(),
            &TransactionOptions { only_query: self.is_only_query() },
        );
        let tx_hash = sn_api_transaction.calculate_transaction_hash(&chain_id, &tx_version)?;

        Ok(starknet_api::executable_transaction::InvokeTransaction {
            tx: sn_api_transaction,
            tx_hash,
        })
    }
}

impl<'de> Deserialize<'de> for BroadcastedDeclareTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let version_raw = value.get("version").ok_or(serde::de::Error::missing_field("version"))?;
        match version_raw.as_str() {
            Some(v) if ["0x1", "0x100000000000000000000000000000001"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid declare transaction v1: {e}"))
                })?;
                Ok(BroadcastedDeclareTransaction::V1(Box::new(unpacked)))
            }
            Some(v) if ["0x2", "0x100000000000000000000000000000002"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid declare transaction v2: {e}"))
                })?;
                Ok(BroadcastedDeclareTransaction::V2(Box::new(unpacked)))
            }
            Some(v) if ["0x3", "0x100000000000000000000000000000003"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid declare transaction v3: {e}"))
                })?;
                Ok(BroadcastedDeclareTransaction::V3(Box::new(unpacked)))
            }
            _ => Err(serde::de::Error::custom(format!(
                "Invalid version of declare transaction: {version_raw}"
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for BroadcastedDeployAccountTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let version_raw = value.get("version").ok_or(serde::de::Error::missing_field("version"))?;
        match version_raw.as_str() {
            Some(v) if ["0x1", "0x100000000000000000000000000000001"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid deploy account transaction v1: {e}"))
                })?;
                Ok(BroadcastedDeployAccountTransaction::V1(unpacked))
            }
            Some(v) if ["0x3", "0x100000000000000000000000000000003"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid deploy account transaction v3: {e}"))
                })?;
                Ok(BroadcastedDeployAccountTransaction::V3(unpacked))
            }
            _ => Err(serde::de::Error::custom(format!(
                "Invalid version of deploy account transaction: {version_raw}"
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for BroadcastedInvokeTransaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let version_raw = value.get("version").ok_or(serde::de::Error::missing_field("version"))?;
        match version_raw.as_str() {
            Some(v) if ["0x1", "0x100000000000000000000000000000001"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid invoke transaction v1: {e}"))
                })?;
                Ok(BroadcastedInvokeTransaction::V1(unpacked))
            }
            Some(v) if ["0x3", "0x100000000000000000000000000000003"].contains(&v) => {
                let unpacked = serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid invoke transaction v3: {e}"))
                })?;
                Ok(BroadcastedInvokeTransaction::V3(unpacked))
            }
            _ => Err(serde::de::Error::custom(format!(
                "Invalid version of invoke transaction: {version_raw}"
            ))),
        }
    }
}

/// Flags that indicate how to simulate a given transaction.
/// By default, the sequencer behavior is replicated locally (enough funds are expected to be in the
/// account, and fee will be deducted from the balance before the simulation of the next
/// transaction). To skip the fee charge, use the SKIP_FEE_CHARGE flag.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulationFlag {
    SkipValidate,
    SkipFeeCharge,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(Deserialize))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallType {
    LibraryCall,
    Call,
    Delegate,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct FunctionInvocation {
    contract_address: ContractAddress,
    entry_point_selector: EntryPointSelector,
    calldata: Calldata,
    caller_address: ContractAddress,
    class_hash: Felt,
    entry_point_type: EntryPointType,
    call_type: CallType,
    result: Vec<Felt>,
    calls: Vec<FunctionInvocation>,
    events: Vec<OrderedEvent>,
    messages: Vec<OrderedMessageToL1>,
    execution_resources: ComputationResources,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]

pub enum TransactionTrace {
    Invoke(InvokeTransactionTrace),
    Declare(DeclareTransactionTrace),
    DeployAccount(DeployAccountTransactionTrace),
    L1Handler(L1HandlerTransactionTrace),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct BlockTransactionTrace {
    pub transaction_hash: Felt,
    pub trace_root: TransactionTrace,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct Reversion {
    pub revert_reason: String, // TODO use blockifier's RevertError
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize))]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ExecutionInvocation {
    Succeeded(FunctionInvocation),
    Reverted(Reversion),
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct InvokeTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub execute_invocation: ExecutionInvocation,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct DeployAccountTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub constructor_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct L1HandlerTransactionTrace {
    pub function_invocation: FunctionInvocation,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct SimulatedTransaction {
    pub transaction_trace: TransactionTrace,
    pub fee_estimation: FeeEstimateWrapper,
}

impl FunctionInvocation {
    pub fn try_from_call_info(
        call_info: &blockifier::execution::call_info::CallInfo,
        state_reader: &mut impl StateReader,
    ) -> DevnetResult<Self> {
        let mut internal_calls: Vec<FunctionInvocation> = vec![];
        let execution_resources = ComputationResources::from(call_info);
        for internal_call in &call_info.inner_calls {
            internal_calls
                .push(FunctionInvocation::try_from_call_info(internal_call, state_reader)?);
        }

        let mut messages: Vec<OrderedMessageToL1> = call_info
            .execution
            .l2_to_l1_messages
            .iter()
            .map(|msg| OrderedMessageToL1::new(msg, call_info.call.caller_address.into()))
            .collect();
        messages.sort_by_key(|msg| msg.order);

        let mut events: Vec<OrderedEvent> =
            call_info.execution.events.iter().map(OrderedEvent::from).collect();
        let contract_address = call_info.call.storage_address;
        events.sort_by_key(|event| event.order);

        // call_info.call.class_hash could be None, so we deduce it from
        // call_info.call.storage_address which is function_call.contract_address
        let class_hash = if let Some(class_hash) = call_info.call.class_hash {
            class_hash
        } else {
            state_reader.get_class_hash_at(contract_address).map_err(|_| {
                ConversionError::InvalidInternalStructure(
                    "class_hash is unxpectedly undefined".into(),
                )
            })?
        };

        Ok(FunctionInvocation {
            contract_address: contract_address.into(),
            entry_point_selector: call_info.call.entry_point_selector.0,
            calldata: call_info.call.calldata.0.to_vec(),
            caller_address: call_info.call.caller_address.into(),
            class_hash: class_hash.0,
            entry_point_type: call_info.call.entry_point_type,
            call_type: match call_info.call.call_type {
                blockifier::execution::entry_point::CallType::Call => CallType::Call,
                blockifier::execution::entry_point::CallType::Delegate => CallType::Delegate,
            },
            result: call_info.execution.retdata.0.clone(),
            calls: internal_calls,
            events,
            messages,
            execution_resources,
        })
    }
}
