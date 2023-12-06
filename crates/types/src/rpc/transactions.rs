use std::collections::HashMap;

use blockifier::execution::call_info::CallInfo;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::TransactionExecutionInfo;
use broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use declare_transaction_v0v1::DeclareTransactionV0V1;
use declare_transaction_v2::DeclareTransactionV2;
use deploy_transaction::DeployTransaction;
use invoke_transaction_v1::InvokeTransactionV1;
use serde::{Deserialize, Deserializer, Serialize};
use starknet_api::block::BlockNumber;
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::transaction::{Fee, Resource, ResourceBoundsMapping, Tip};
use starknet_rs_core::types::{BlockId, ExecutionResult, TransactionFinalityStatus};
use starknet_rs_crypto::poseidon_hash_many;
use starknet_rs_ff::FieldElement;

use self::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use self::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use self::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use self::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
use self::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use self::declare_transaction_v3::DeclareTransactionV3;
use self::deploy_account_transaction_v1::DeployAccountTransactionV1;
use self::deploy_account_transaction_v3::DeployAccountTransactionV3;
use self::invoke_transaction_v3::InvokeTransactionV3;
use super::estimate_message_fee::FeeEstimateWrapper;
use super::state::ThinStateDiff;
use super::transaction_receipt::{ExecutionResources, OrderedMessageToL1};
use crate::constants::{
    BITWISE_BUILTIN_NAME, EC_OP_BUILTIN_NAME, HASH_BUILTIN_NAME, KECCAK_BUILTIN_NAME, N_STEPS,
    POSEIDON_BUILTIN_NAME, RANGE_CHECK_BUILTIN_NAME, SIGNATURE_BUILTIN_NAME,
};
use crate::contract_address::ContractAddress;
use crate::emitted_event::{Event, OrderedEvent};
use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::felt::{
    BlockHash, Calldata, EntryPointSelector, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transaction_receipt::{
    CommonTransactionReceipt, MaybePendingProperties, TransactionOutput,
};
use crate::serde_helpers::resource_bounds_mapping::deserialize_by_converting_keys_to_uppercase;

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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Transactions {
    Hashes(Vec<TransactionHash>),
    Full(Vec<Transaction>),
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
#[serde(tag = "type")]
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

impl Transaction {
    pub fn get_type(&self) -> TransactionType {
        match self {
            Transaction::Declare(_) => TransactionType::Declare,
            Transaction::DeployAccount(_) => TransactionType::DeployAccount,
            Transaction::Deploy(_) => TransactionType::Deploy,
            Transaction::Invoke(_) => TransactionType::Invoke,
            Transaction::L1Handler(_) => TransactionType::L1Handler,
        }
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            Transaction::Declare(tx) => tx.get_transaction_hash(),
            Transaction::L1Handler(tx) => tx.get_transaction_hash(),
            Transaction::DeployAccount(tx) => tx.get_transaction_hash(),
            Transaction::Invoke(tx) => tx.get_transaction_hash(),
            Transaction::Deploy(tx) => tx.get_transaction_hash(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_common_receipt(
        &self,
        transaction_events: &[Event],
        block_hash: Option<&BlockHash>,
        block_number: Option<BlockNumber>,
        execution_result: &ExecutionResult,
        finality_status: TransactionFinalityStatus,
        actual_fee: Fee,
        execution_info: &TransactionExecutionInfo,
    ) -> CommonTransactionReceipt {
        let r#type = self.get_type();

        fn get_memory_holes_from_call_info(call_info: &Option<CallInfo>) -> usize {
            if let Some(call) = call_info { call.vm_resources.n_memory_holes } else { 0 }
        }

        fn get_resource_from_execution_info(
            execution_info: &TransactionExecutionInfo,
            resource_name: &str,
        ) -> Felt {
            let resource =
                execution_info.actual_resources.0.get(resource_name).cloned().unwrap_or_default();
            Felt::from(resource as u128)
        }

        let total_memory_holes = get_memory_holes_from_call_info(&execution_info.execute_call_info)
            + get_memory_holes_from_call_info(&execution_info.validate_call_info)
            + get_memory_holes_from_call_info(&execution_info.fee_transfer_call_info);

        let execution_resources = ExecutionResources {
            steps: get_resource_from_execution_info(execution_info, N_STEPS),
            memory_holes: Felt::from(total_memory_holes as u128),
            range_check_builtin_applications: get_resource_from_execution_info(
                execution_info,
                RANGE_CHECK_BUILTIN_NAME,
            ),
            pedersen_builtin_applications: get_resource_from_execution_info(
                execution_info,
                HASH_BUILTIN_NAME,
            ),
            poseidon_builtin_applications: get_resource_from_execution_info(
                execution_info,
                POSEIDON_BUILTIN_NAME,
            ),
            ec_op_builtin_applications: get_resource_from_execution_info(
                execution_info,
                EC_OP_BUILTIN_NAME,
            ),
            ecdsa_builtin_applications: get_resource_from_execution_info(
                execution_info,
                SIGNATURE_BUILTIN_NAME,
            ),
            bitwise_builtin_applications: get_resource_from_execution_info(
                execution_info,
                BITWISE_BUILTIN_NAME,
            ),
            keccak_builtin_applications: get_resource_from_execution_info(
                execution_info,
                KECCAK_BUILTIN_NAME,
            ),
        };

        let output = TransactionOutput {
            actual_fee,
            messages_sent: Vec::new(), // TODO wrong
            events: transaction_events.to_vec(),
        };

        let maybe_pending_properties =
            MaybePendingProperties { block_number, block_hash: block_hash.cloned() };

        CommonTransactionReceipt {
            r#type,
            transaction_hash: *self.get_transaction_hash(),
            output,
            execution_status: execution_result.clone(),
            finality_status,
            maybe_pending_properties,
            execution_resources,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclareTransaction {
    Version0(DeclareTransactionV0V1),
    Version1(DeclareTransactionV0V1),
    Version2(DeclareTransactionV2),
    Version3(DeclareTransactionV3),
}

impl DeclareTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            DeclareTransaction::Version0(tx) => tx.get_transaction_hash(),
            DeclareTransaction::Version1(tx) => tx.get_transaction_hash(),
            DeclareTransaction::Version2(tx) => tx.get_transaction_hash(),
            DeclareTransaction::Version3(tx) => tx.get_transaction_hash(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct InvokeTransactionV0 {
    pub transaction_hash: TransactionHash,
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

impl InvokeTransactionV0 {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InvokeTransaction {
    Version0(InvokeTransactionV0),
    Version1(InvokeTransactionV1),
    Version3(InvokeTransactionV3),
}

impl InvokeTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            InvokeTransaction::Version0(tx) => tx.get_transaction_hash(),
            InvokeTransaction::Version1(tx) => tx.get_transaction_hash(),
            InvokeTransaction::Version3(tx) => tx.get_transaction_hash(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DeployAccountTransaction {
    Version1(Box<DeployAccountTransactionV1>),
    Version3(Box<DeployAccountTransactionV3>),
}

impl DeployAccountTransaction {
    pub fn get_contract_address(&self) -> &ContractAddress {
        match self {
            DeployAccountTransaction::Version1(tx) => &tx.contract_address,
            DeployAccountTransaction::Version3(tx) => tx.get_contract_address(),
        }
    }

    pub fn get_transaction_hash(&self) -> &TransactionHash {
        match self {
            DeployAccountTransaction::Version1(tx) => tx.get_transaction_hash(),
            DeployAccountTransaction::Version3(tx) => tx.get_transaction_hash(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct L1HandlerTransaction {
    pub transaction_hash: TransactionHash,
    pub version: TransactionVersion,
    pub nonce: Nonce,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
}

impl L1HandlerTransaction {
    pub fn get_transaction_hash(&self) -> &TransactionHash {
        &self.transaction_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommon {
    pub max_fee: Fee,
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
}

/// Common fields for all transaction type of version 3
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedTransactionCommonV3 {
    pub version: TransactionVersion,
    pub signature: TransactionSignature,
    pub nonce: Nonce,
    #[serde(deserialize_with = "deserialize_by_converting_keys_to_uppercase")]
    pub resource_bounds: ResourceBoundsMapping,
    pub tip: Tip,
    pub paymaster_data: Vec<Felt>,
    pub nonce_data_availability_mode: DataAvailabilityMode,
    pub fee_data_availability_mode: DataAvailabilityMode,
}

impl BroadcastedTransactionCommonV3 {
    /// Checks if total accumulated fee of resource_bounds is equal to 0
    pub fn is_max_fee_zero_value(&self) -> bool {
        let fee_total_value: u128 = self
            .resource_bounds
            .0
            .values()
            .fold(0u128, |acc, el| acc + (el.max_price_per_unit * (el.max_amount as u128)));

        fee_total_value == 0
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

        let ordered_resources = vec![Resource::L1Gas, Resource::L2Gas];

        for resource in ordered_resources {
            if let Some(resource_bound) = self.resource_bounds.0.get(&resource) {
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
                    resource_bound.max_amount.to_be_bytes().as_slice(),
                    resource_bound.max_price_per_unit.to_be_bytes().as_slice(),
                ]
                .into_iter()
                .flatten()
                .copied()
                .collect();

                array.push(FieldElement::from_byte_slice_be(bytes.as_slice())?);
            }
        }

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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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
        only_query: bool,
    ) -> DevnetResult<blockifier::transaction::account_transaction::AccountTransaction> {
        let blockifier_transaction = match self {
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(invoke_txn)) => {
                let blockifier_invoke_txn =
                    invoke_txn.create_blockifier_invoke_transaction(chain_id, only_query)?;
                AccountTransaction::Invoke(blockifier_invoke_txn)
            }
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(invoke_txn)) => {
                let blockifier_invoke_txn =
                    invoke_txn.create_blockifier_invoke_transaction(chain_id, only_query)?;
                AccountTransaction::Invoke(blockifier_invoke_txn)
            }
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
                AccountTransaction::Declare(
                    declare_v3.create_blockifier_declare(chain_id, only_query)?,
                )
            }
            BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V1(
                deploy_account_v1,
            )) => AccountTransaction::DeployAccount(
                deploy_account_v1.create_blockifier_deploy_account(chain_id, only_query)?,
            ),
            BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V3(
                deploy_account_v3,
            )) => AccountTransaction::DeployAccount(
                deploy_account_v3.create_blockifier_deploy_account(chain_id, only_query)?,
            ),
        };

        Ok(blockifier_transaction)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeclareTransaction {
    V1(Box<BroadcastedDeclareTransactionV1>),
    V2(Box<BroadcastedDeclareTransactionV2>),
    V3(Box<BroadcastedDeclareTransactionV3>),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedDeployAccountTransaction {
    V1(BroadcastedDeployAccountTransactionV1),
    V3(BroadcastedDeployAccountTransactionV3),
}
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BroadcastedInvokeTransaction {
    V1(BroadcastedInvokeTransactionV1),
    V3(BroadcastedInvokeTransactionV3),
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
            _ => Err(serde::de::Error::custom(format!(
                "Invalid version of declare transaction: {version_raw}"
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
pub struct FunctionInvocation {
    #[serde(flatten)]
    function_call: FunctionCall,
    caller_address: ContractAddress,
    class_hash: Felt,
    entry_point_type: EntryPointType,
    call_type: CallType,
    result: Vec<Felt>,
    calls: Vec<FunctionInvocation>,
    events: Vec<OrderedEvent>,
    messages: Vec<OrderedMessageToL1>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployAccountTransactionTrace {
    pub validate_invocation: Option<FunctionInvocation>,
    pub constructor_invocation: Option<FunctionInvocation>,
    pub fee_transfer_invocation: Option<FunctionInvocation>,
    pub state_diff: Option<ThinStateDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulatedTransaction {
    pub transaction_trace: TransactionTrace,
    pub fee_estimation: FeeEstimateWrapper,
}

impl FunctionInvocation {
    pub fn try_from_call_info(
        mut call_info: blockifier::execution::call_info::CallInfo,
        address_to_class_hash: &HashMap<ContractAddress, Felt>,
    ) -> DevnetResult<Self> {
        let mut internal_calls: Vec<FunctionInvocation> = vec![];
        for internal_call in call_info.inner_calls {
            internal_calls.push(FunctionInvocation::try_from_call_info(
                internal_call,
                address_to_class_hash,
            )?);
        }

        // the logic for getting the sorted l2-l1 messages
        // is creating an array with enough room for all objects + 1
        // then based on the order we use this index

        call_info.execution.l2_to_l1_messages.sort_by_key(|msg| msg.order);

        let messages: Vec<OrderedMessageToL1> = call_info
            .execution
            .l2_to_l1_messages
            .into_iter()
            .map(|msg| OrderedMessageToL1::new(msg, call_info.call.caller_address.into()))
            .collect();

        call_info.execution.events.sort_by_key(|event| event.order);

        let events: Vec<OrderedEvent> = call_info
            .execution
            .events
            .into_iter()
            .map(|event| OrderedEvent::from(&event))
            .collect();

        let function_call = FunctionCall {
            contract_address: call_info.call.storage_address.into(),
            entry_point_selector: call_info.call.entry_point_selector.0.into(),
            calldata: call_info.call.calldata.0.iter().map(|f| Felt::from(*f)).collect(),
        };

        // call_info.call.class_hash could be None, so we deduce it from
        // call_info.call.storage_address which is function_call.contract_address
        let class_hash = if let Some(class_hash) = call_info.call.class_hash {
            class_hash.into()
        } else {
            address_to_class_hash
                .get(&function_call.contract_address)
                .ok_or(ConversionError::InvalidInternalStructure(
                    "class_hash is unexpectedly undefined".into(),
                ))
                .cloned()?
        };

        Ok(FunctionInvocation {
            function_call,
            caller_address: call_info.call.caller_address.into(),
            class_hash,
            entry_point_type: call_info.call.entry_point_type,
            call_type: match call_info.call.call_type {
                blockifier::execution::entry_point::CallType::Call => CallType::Call,
                blockifier::execution::entry_point::CallType::Delegate => CallType::Delegate,
            },
            result: call_info.execution.retdata.0.into_iter().map(Felt::from).collect(),
            calls: internal_calls,
            events,
            messages,
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
              "L2_GAS": {
                "max_amount": "0x0",
                "max_price_per_unit": "0x0"
              },
              "L1_GAS": {
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
        println!("{:x}", common_fields_hash);

        let expected_hash: FieldElement = FieldElement::from_hex_be(
            "0x07be65f04548dfe645c70f07d1f8ead572c09e0e6e125c47d4cc22b4de3597cc",
        )
        .unwrap();

        assert_eq!(common_fields_hash, expected_hash);
    }
}
