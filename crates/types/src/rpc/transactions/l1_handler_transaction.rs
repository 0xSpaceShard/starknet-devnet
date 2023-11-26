use std::sync::Arc;

use blockifier::transaction::transactions::L1HandlerTransaction as BlockifierL1HandlerTransaction;
use starknet_api::core::{
    ContractAddress as ApiContractAddress, EntryPointSelector as ApiEntryPointSelector,
    Nonce as ApiNonce,
};
use starknet_api::transaction::{
    Calldata as ApiCalldata, Fee as ApiFee, L1HandlerTransaction as ApiL1HandlerTransaction,
    TransactionHash as ApiTransactionHash, TransactionVersion as ApiTransactionVersion,
};
use starknet_rs_core::crypto::compute_hash_on_elements;
use starknet_rs_core::types::FieldElement;

use crate::error::{ConversionError, DevnetResult, Error};
use crate::felt::Felt;
use crate::rpc::messaging::MessageToL2;
use crate::rpc::transactions::L1HandlerTransaction;
use crate::traits::HashProducer;

/// Cairo string for "l1_handler"
const PREFIX_L1_HANDLER: FieldElement = FieldElement::from_mont([
    1365666230910873368,
    18446744073708665300,
    18446744073709551615,
    157895833347907735,
]);

impl L1HandlerTransaction {
    /// Instanciates a new `L1HandlerTransaction`.
    /// TODO: a bit too much arguments here...
    pub fn with_hash(mut self, chain_id: Felt) -> Self {
        self.transaction_hash = self.compute_hash(chain_id);
        self
    }

    /// Computes the hash of a `L1HandlerTransaction`.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID.
    pub fn compute_hash(&self, chain_id: Felt) -> Felt {
        assert_eq!(
            self.version,
            FieldElement::ZERO.into(),
            "L1 handler transaction only supports version 0"
        );

        // No fee on L2 for L1 handler transaction.
        let fee = FieldElement::ZERO;

        compute_hash_on_elements(&[
            PREFIX_L1_HANDLER,
            self.version.into(),
            self.contract_address.into(),
            self.entry_point_selector.into(),
            compute_hash_on_elements(
                &self
                    .calldata
                    .iter()
                    .map(|felt| FieldElement::from(*felt))
                    .collect::<Vec<FieldElement>>(),
            ),
            fee,
            chain_id.into(),
            self.nonce.into(),
        ])
        .into()
    }

    /// Creates a blockifier version of `L1HandlerTransaction`.
    pub fn create_blockifier_transaction(&self) -> DevnetResult<BlockifierL1HandlerTransaction> {
        let version: Felt = 0_u128.into();

        let transaction = BlockifierL1HandlerTransaction {
            tx: ApiL1HandlerTransaction {
                contract_address: ApiContractAddress::try_from(self.contract_address)?,
                entry_point_selector: ApiEntryPointSelector(self.entry_point_selector.into()),
                calldata: ApiCalldata(Arc::new(self.calldata.iter().map(|f| f.into()).collect())),
                nonce: ApiNonce(self.nonce.into()),
                version: ApiTransactionVersion(version.into()),
            },
            paid_fee_on_l1: ApiFee(self.paid_fee_on_l1),
            tx_hash: ApiTransactionHash(self.transaction_hash.into()),
        };

        Ok(transaction)
    }

    /// Converts a `MessageToL2` into a `L1HandlerTransaction`.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to be converted.
    /// * `chain_id` - The L1 node chain id.
    pub fn try_from_message_to_l2(message: MessageToL2) -> DevnetResult<Self> {
        // `impl TryFrom` is not used due to the fact that chain_id is required.
        let paid_fee_on_l1: u128 =
            message.paid_fee_on_l1.try_into().map_err(|_| ConversionError::OutOfRangeError)?;

        let mut calldata = vec![message.l1_contract_address.into()];
        for u in message.payload {
            calldata.push(u);
        }

        Ok(Self {
            contract_address: message.l2_contract_address,
            entry_point_selector: message.entry_point_selector,
            calldata,
            nonce: message.nonce,
            paid_fee_on_l1,
            ..Default::default()
        })
    }
}

// TODO: for this version I didn't impl `HashProducer`
// as the chain_id is not present in the transaction itself.
// Should we keep the chain_id somewhere in the devnet `L1HandlerTransaction`?
// Or we don't want HashProducder on `L1HandlerTransaction` and the `new` is fine?
impl HashProducer for L1HandlerTransaction {
    type Error = Error;

    fn generate_hash(&self) -> DevnetResult<Felt> {
        assert_eq!(
            self.version,
            FieldElement::ZERO.into(),
            "L1 handler transaction only supports version 0"
        );

        // No fee on L2 for L1 handler transaction.
        let fee = FieldElement::ZERO;

        Ok(compute_hash_on_elements(&[
            PREFIX_L1_HANDLER,
            self.version.into(),
            self.contract_address.into(),
            self.entry_point_selector.into(),
            compute_hash_on_elements(
                &self
                    .calldata
                    .iter()
                    .map(|felt| FieldElement::from(*felt))
                    .collect::<Vec<FieldElement>>(),
            ),
            fee,
            // TODO: How to get the chain_id at this point?
            // Or should this be computed in an other fashion?
            // chain_id.into(),
            0_u128.into(),
            self.nonce.into(),
        ])
        .into())
    }
}
