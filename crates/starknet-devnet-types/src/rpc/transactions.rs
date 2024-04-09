use std::collections::BTreeMap;
use std::sync::Arc;

use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::TransactionExecutionInfo;
use broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use declare_transaction_v0v1::DeclareTransactionV0V1;
use declare_transaction_v2::DeclareTransactionV2;
use deploy_transaction::DeployTransaction;
use invoke_transaction_v1::InvokeTransactionV1;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_api::block::BlockNumber;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::{Fee, Resource, Tip};
use starknet_rs_core::crypto::compute_hash_on_elements;
use starknet_rs_core::types::{
    BlockId, ExecutionResult, ResourceBounds, ResourceBoundsMapping, TransactionFinalityStatus,
};
use starknet_rs_crypto::poseidon_hash_many;
use starknet_rs_ff::FieldElement;

use self::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use self::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use self::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use self::broadcasted_invoke_transaction_v1::{BroadcastedInvokeTransactionV1, PREFIX_INVOKE};
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
use crate::emitted_event::{Event, OrderedEvent};
use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::felt::{
    BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transaction_receipt::{CommonTransactionReceipt, MaybePendingProperties};
use crate::utils::into_vec;
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

/// number of bits to be shifted when encoding the data availability mode into `FieldElement` type
const DATA_AVAILABILITY_MODE_BITS: u8 = 32;

pub mod l1_handler_transaction;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Transactions {
    Hashes(Vec<TransactionHash>),
    Full(Vec<TransactionWithHash>),
    FullWithReceipts(Vec<TransactionWithReceipt>),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Default)]
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Transaction {
    #[serde(rename = "DECLARE")]
    Declare(DeclareTransaction),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(DeployAccountTransaction),
    #[serde(rename = "DEPLOY")]
    Deploy(DeployTransaction),
    #[serde(rename = "INVOKE")]
    Invoke(InvokeTransaction),
    #[serde(rename = "L1_HANDLER")]
    L1Handler(L1HandlerTransaction),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionWithReceipt {
    pub receipt: TransactionReceipt,
    pub transaction: Transaction,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclareTransaction {
    V1(DeclareTransactionV0V1),
    V2(DeclareTransactionV2),
    V3(DeclareTransactionV3),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InvokeTransaction {
    V1(InvokeTransactionV1),
    V3(InvokeTransactionV3),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsChunk {
    pub events: Vec<crate::emitted_event::EmittedEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

impl From<&ResourceBoundsWrapper> for starknet_api::transaction::ResourceBoundsMapping {
    fn from(value: &ResourceBoundsWrapper) -> Self {
        starknet_api::transaction::ResourceBoundsMapping(BTreeMap::from([
            (
                Resource::L1Gas,
                starknet_api::transaction::ResourceBounds {
                    max_amount: value.inner.l1_gas.max_amount,
                    max_price_per_unit: value.inner.l1_gas.max_price_per_unit,
                },
            ),
            (
                Resource::L2Gas,
                starknet_api::transaction::ResourceBounds {
                    max_amount: value.inner.l2_gas.max_amount,
                    max_price_per_unit: value.inner.l2_gas.max_price_per_unit,
                },
            ),
        ]))
    }
}

impl BroadcastedTransactionCommonV3 {
    /// Checks if total accumulated fee of resource_bounds is equal to 0
    pub fn is_max_fee_zero_value(&self) -> bool {
        (self.resource_bounds.inner.l1_gas.max_amount as u128)
            * self.resource_bounds.inner.l1_gas.max_price_per_unit
            == 0
            && (self.resource_bounds.inner.l2_gas.max_amount as u128)
                * self.resource_bounds.inner.l2_gas.max_price_per_unit
                == 0
    }
    /// Returns an array of FieldElements that reflects the `common_tx_fields` according to SNIP-8(https://github.com/starknet-io/SNIPs/blob/main/SNIPS/snip-8.md/#protocol-changes).
    ///
    /// # Arguments
    /// tx_prefix - the prefix of the transaction hash
    /// chain_id - the chain id of the network the transaction is broadcasted to
    /// address - the address of the sender
    pub(crate) fn common_fields_for_hash(
        &self,
        tx_prefix: FieldElement,
        chain_id: FieldElement,
        address: FieldElement,
    ) -> Result<Vec<FieldElement>, Error> {
        let array: Vec<FieldElement> = vec![
            tx_prefix,                                                        // TX_PREFIX
            self.version.into(),                                              // version
            address,                                                          // address
            poseidon_hash_many(self.get_resource_bounds_array()?.as_slice()), /* h(tip, resource_bounds_for_fee) */
            poseidon_hash_many(
                self.paymaster_data
                    .iter()
                    .map(|f| FieldElement::from(*f))
                    .collect::<Vec<FieldElement>>()
                    .as_slice(),
            ), // h(paymaster_data)
            chain_id,                                                         // chain_id
            self.nonce.into(),                                                // nonce
            self.get_data_availability_modes_field_element(), /* nonce_data_availabilty ||
                                                               * fee_data_availability_mode */
        ];

        Ok(array)
    }

    /// Returns the array of FieldElements that reflects (tip, resource_bounds_for_fee) from SNIP-8
    pub(crate) fn get_resource_bounds_array(&self) -> Result<Vec<FieldElement>, Error> {
        let mut array = Vec::<FieldElement>::new();
        array.push(FieldElement::from(self.tip.0));

        fn field_element_from_resource_bounds(
            resource: Resource,
            resource_bounds: &ResourceBounds,
        ) -> Result<FieldElement, Error> {
            let resource_name_as_json_string =
                serde_json::to_value(resource).map_err(JsonError::SerdeJsonError)?;

            let resource_name_bytes = resource_name_as_json_string
                .as_str()
                .ok_or(Error::JsonError(JsonError::Custom {
                    msg: "resource name is not a string".into(),
                }))?
                .as_bytes();

            // (resource||max_amount||max_price_per_unit) from SNIP-8 https://github.com/starknet-io/SNIPs/blob/main/SNIPS/snip-8.md#protocol-changes
            let bytes: Vec<u8> = [
                resource_name_bytes,
                resource_bounds.max_amount.to_be_bytes().as_slice(),
                resource_bounds.max_price_per_unit.to_be_bytes().as_slice(),
            ]
            .into_iter()
            .flatten()
            .copied()
            .collect();

            Ok(FieldElement::from_byte_slice_be(bytes.as_slice())?)
        }
        array.push(field_element_from_resource_bounds(
            Resource::L1Gas,
            &self.resource_bounds.inner.l1_gas,
        )?);
        array.push(field_element_from_resource_bounds(
            Resource::L2Gas,
            &self.resource_bounds.inner.l2_gas,
        )?);

        Ok(array)
    }

    /// Returns FieldElement that encodes the data availability modes of the transaction
    pub(crate) fn get_data_availability_modes_field_element(&self) -> FieldElement {
        fn get_data_availability_mode_value_as_u64(
            data_availability_mode: DataAvailabilityMode,
        ) -> u64 {
            match data_availability_mode {
                DataAvailabilityMode::L1 => 0,
                DataAvailabilityMode::L2 => 1,
            }
        }

        let da_mode = get_data_availability_mode_value_as_u64(self.nonce_data_availability_mode)
            << DATA_AVAILABILITY_MODE_BITS;
        let da_mode =
            da_mode + get_data_availability_mode_value_as_u64(self.fee_data_availability_mode);

        FieldElement::from(da_mode)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum BroadcastedTransaction {
    #[serde(rename = "INVOKE")]
    Invoke(BroadcastedInvokeTransaction),
    #[serde(rename = "DECLARE")]
    Declare(BroadcastedDeclareTransaction),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(BroadcastedDeployAccountTransaction),
}

impl BroadcastedTransaction {
    pub fn to_blockifier_account_transaction(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<blockifier::transaction::account_transaction::AccountTransaction> {
        let blockifier_transaction = match self {
            BroadcastedTransaction::Invoke(invoke_txn) => AccountTransaction::Invoke(
                invoke_txn.create_blockifier_invoke_transaction(chain_id)?,
            ),
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(declare_v1)) => {
                let class_hash = declare_v1.generate_class_hash()?;
                let transaction_hash =
                    declare_v1.calculate_transaction_hash(&chain_id, &class_hash)?;
                AccountTransaction::Declare(
                    declare_v1.create_blockifier_declare(class_hash, transaction_hash)?,
                )
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(declare_v2)) => {
                AccountTransaction::Declare(declare_v2.create_blockifier_declare(chain_id)?)
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(declare_v3)) => {
                AccountTransaction::Declare(declare_v3.create_blockifier_declare(chain_id)?)
            }
            BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V1(
                deploy_account_v1,
            )) => AccountTransaction::DeployAccount(
                deploy_account_v1.create_blockifier_deploy_account(chain_id)?,
            ),
            BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V3(
                deploy_account_v3,
            )) => AccountTransaction::DeployAccount(
                deploy_account_v3.create_blockifier_deploy_account(chain_id)?,
            ),
        };

        Ok(blockifier_transaction)
    }

    pub fn get_type(&self) -> TransactionType {
        match self {
            BroadcastedTransaction::Invoke(_) => TransactionType::Invoke,
            BroadcastedTransaction::Declare(_) => TransactionType::Declare,
            BroadcastedTransaction::DeployAccount(_) => TransactionType::DeployAccount,
        }
    }

    pub(crate) fn is_query_only_version(version: Felt) -> bool {
        FieldElement::from(version) > QUERY_VERSION_OFFSET
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
    V3(Box<BroadcastedDeclareTransactionV3>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeployAccountTransaction {
    V1(BroadcastedDeployAccountTransactionV1),
    V3(BroadcastedDeployAccountTransactionV3),
}
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum BroadcastedInvokeTransaction {
    V1(BroadcastedInvokeTransactionV1),
    V3(BroadcastedInvokeTransactionV3),
}

impl BroadcastedInvokeTransaction {
    /// Creates a blockifier invoke transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    pub fn create_blockifier_invoke_transaction(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<blockifier::transaction::transactions::InvokeTransaction> {
        let (transaction_hash, sn_api_transaction, version) = match self {
            BroadcastedInvokeTransaction::V1(v1) => {
                let txn_hash: Felt = compute_hash_on_elements(&[
                    PREFIX_INVOKE,
                    v1.common.version.into(), // version
                    v1.sender_address.into(),
                    FieldElement::ZERO, // entry_point_selector
                    compute_hash_on_elements(
                        &v1.calldata
                            .iter()
                            .map(|felt| FieldElement::from(*felt))
                            .collect::<Vec<FieldElement>>(),
                    ),
                    v1.common.max_fee.0.into(),
                    chain_id.into(),
                    v1.common.nonce.into(),
                ])
                .into();

                let sn_api_transaction = starknet_api::transaction::InvokeTransactionV1 {
                    max_fee: v1.common.max_fee,
                    signature: starknet_api::transaction::TransactionSignature(
                        v1.common.signature.iter().map(|f| f.into()).collect(),
                    ),
                    nonce: starknet_api::core::Nonce(v1.common.nonce.into()),
                    sender_address: v1.sender_address.try_into()?,
                    calldata: starknet_api::transaction::Calldata(Arc::new(
                        v1.calldata.iter().map(StarkFelt::from).collect::<Vec<StarkFelt>>(),
                    )),
                };

                (
                    txn_hash,
                    starknet_api::transaction::InvokeTransaction::V1(sn_api_transaction),
                    v1.common.version,
                )
            }
            BroadcastedInvokeTransaction::V3(v3) => {
                let txn_hash = v3.calculate_transaction_hash(chain_id)?;

                let sn_api_transaction = starknet_api::transaction::InvokeTransactionV3 {
                    resource_bounds: (&v3.common.resource_bounds).into(),
                    tip: v3.common.tip,
                    signature: starknet_api::transaction::TransactionSignature(into_vec(
                        &v3.common.signature,
                    )),
                    nonce: starknet_api::core::Nonce(v3.common.nonce.into()),
                    sender_address: v3.sender_address.try_into()?,
                    calldata: starknet_api::transaction::Calldata(Arc::new(
                        v3.calldata.iter().map(StarkFelt::from).collect::<Vec<StarkFelt>>(),
                    )),
                    nonce_data_availability_mode: v3.common.nonce_data_availability_mode,
                    fee_data_availability_mode: v3.common.fee_data_availability_mode,
                    paymaster_data: starknet_api::transaction::PaymasterData(
                        v3.common.paymaster_data.iter().map(|f| f.into()).collect(),
                    ),
                    account_deployment_data: starknet_api::transaction::AccountDeploymentData(
                        v3.account_deployment_data.iter().map(|f| f.into()).collect(),
                    ),
                };

                (
                    txn_hash,
                    starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction),
                    v3.common.version,
                )
            }
        };

        let only_query = BroadcastedTransaction::is_query_only_version(version);

        Ok(blockifier::transaction::transactions::InvokeTransaction {
            tx: sn_api_transaction,
            tx_hash: starknet_api::transaction::TransactionHash(transaction_hash.into()),
            only_query,
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
pub enum SimulationFlag {
    #[serde(rename = "SKIP_VALIDATE")]
    SkipValidate,
    #[serde(rename = "SKIP_FEE_CHARGE")]
    SkipFeeCharge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    #[serde(rename = "LIBRARY_CALL")]
    LibraryCall,
    #[serde(rename = "CALL")]
    Call,
    #[serde(rename = "DELEGATE")]
    Delegate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionTrace {
    #[serde(rename = "INVOKE")]
    Invoke(InvokeTransactionTrace),
    #[serde(rename = "DECLARE")]
    Declare(DeclareTransactionTrace),
    #[serde(rename = "DEPLOY_ACCOUNT")]
    DeployAccount(DeployAccountTransactionTrace),
    #[serde(rename = "L1_HANDLER")]
    L1Handler(L1HandlerTransactionTrace),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTransactionTrace {
    pub transaction_hash: Felt,
    pub trace_root: TransactionTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reversion {
    pub revert_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecutionInvocation {
    Succeeded(FunctionInvocation),
    Reverted(Reversion),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub execute_invocation: ExecutionInvocation,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployAccountTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub constructor_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L1HandlerTransactionTrace {
    pub function_invocation: FunctionInvocation,
    pub state_diff: Option<ThinStateDiff>,
    pub execution_resources: ExecutionResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
            entry_point_selector: call_info.call.entry_point_selector.0.into(),
            calldata: call_info.call.calldata.0.iter().map(|f| Felt::from(*f)).collect(),
            caller_address: call_info.call.caller_address.into(),
            class_hash: class_hash.into(),
            entry_point_type: call_info.call.entry_point_type,
            call_type: match call_info.call.call_type {
                blockifier::execution::entry_point::CallType::Call => CallType::Call,
                blockifier::execution::entry_point::CallType::Delegate => CallType::Delegate,
            },
            result: call_info.execution.retdata.0.clone().into_iter().map(Felt::from).collect(),
            calls: internal_calls,
            events,
            messages,
            execution_resources,
        })
    }
}

#[cfg(test)]
mod tests {
    use starknet_rs_crypto::poseidon_hash_many;
    use starknet_rs_ff::FieldElement;

    use super::BroadcastedTransactionCommonV3;

    #[test]
    fn test_dummy_transaction_hash_taken_from_papyrus() {
        let txn_json_str = r#"{
            "signature": ["0x3", "0x4"],
            "version": "0x3",
            "nonce": "0x9",
            "sender_address": "0x12fd538",
            "constructor_calldata": ["0x21b", "0x151"],
            "nonce_data_availability_mode": "L1",
            "fee_data_availability_mode": "L1",
            "resource_bounds": {
              "l2_gas": {
                "max_amount": "0x0",
                "max_price_per_unit": "0x0"
              },
              "l1_gas": {
                "max_amount": "0x7c9",
                "max_price_per_unit": "0x1"
              }
            },
            "tip": "0x0",
            "paymaster_data": [],
            "account_deployment_data": [],
            "calldata": [
              "0x11",
              "0x26"
            ]
          }"#;

        let common_fields =
            serde_json::from_str::<BroadcastedTransactionCommonV3>(txn_json_str).unwrap();
        let common_fields_hash =
            poseidon_hash_many(&common_fields.get_resource_bounds_array().unwrap());

        let expected_hash: FieldElement = FieldElement::from_hex_be(
            "0x07be65f04548dfe645c70f07d1f8ead572c09e0e6e125c47d4cc22b4de3597cc",
        )
        .unwrap();

        assert_eq!(common_fields_hash, expected_hash);
    }
}
