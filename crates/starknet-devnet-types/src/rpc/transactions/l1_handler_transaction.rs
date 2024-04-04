use std::sync::Arc;

use blockifier::transaction::transactions::L1HandlerTransaction as BlockifierL1HandlerTransaction;
use serde::{Deserialize, Serialize};
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

use super::{deserialize_paid_fee_on_l1, serialize_paid_fee_on_l1};
use crate::contract_address::ContractAddress;
use crate::error::{ConversionError, DevnetResult, Error};
use crate::felt::{Calldata, EntryPointSelector, Felt, Nonce, TransactionVersion};
use crate::rpc::messaging::MessageToL2;

/// Cairo string for "l1_handler"
const PREFIX_L1_HANDLER: FieldElement = FieldElement::from_mont([
    1365666230910873368,
    18446744073708665300,
    18446744073709551615,
    157895833347907735,
]);

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct L1HandlerTransaction {
    pub version: TransactionVersion,
    pub nonce: Nonce,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
    #[serde(
        serialize_with = "serialize_paid_fee_on_l1",
        deserialize_with = "deserialize_paid_fee_on_l1"
    )]
    pub paid_fee_on_l1: u128,
}

impl L1HandlerTransaction {
    /// Computes the hash of a `L1HandlerTransaction`.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID.
    pub fn compute_hash(&self, chain_id: Felt) -> Felt {
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
    pub fn create_blockifier_transaction(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<BlockifierL1HandlerTransaction> {
        let transaction = BlockifierL1HandlerTransaction {
            tx: ApiL1HandlerTransaction {
                contract_address: ApiContractAddress::try_from(self.contract_address)?,
                entry_point_selector: ApiEntryPointSelector(self.entry_point_selector.into()),
                calldata: ApiCalldata(Arc::new(self.calldata.iter().map(|f| f.into()).collect())),
                nonce: ApiNonce(self.nonce.into()),
                version: ApiTransactionVersion(self.version.into()),
            },
            paid_fee_on_l1: ApiFee(self.paid_fee_on_l1),
            tx_hash: ApiTransactionHash(self.compute_hash(chain_id).into()),
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
        let paid_fee_on_l1: u128 = message.paid_fee_on_l1.try_into().map_err(|_| {
            ConversionError::OutOfRangeError(format!(
                "paid_fee_on_l1 is expected to be a u128 value, found: {:?}",
                message.paid_fee_on_l1,
            ))
        })?;

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
            // Currently, only version 0 is supported, which
            // is ensured by default initialization.
            ..Default::default()
        })
    }
}

impl TryFrom<&L1HandlerTransaction> for MessageToL2 {
    type Error = Error;

    fn try_from(value: &L1HandlerTransaction) -> Result<Self, Self::Error> {
        let l1_contract_address = value.calldata.get(0).ok_or(Error::ConversionError(
            ConversionError::InvalidInternalStructure(
                "L1HandlerTransaction calldata is expected to have at least one element"
                    .to_string(),
            ),
        ))?;

        let payload = value.calldata[1..].to_vec();
        Ok(MessageToL2 {
            l2_contract_address: value.contract_address,
            entry_point_selector: value.entry_point_selector,
            l1_contract_address: ContractAddress::new(*l1_contract_address)?,
            payload,
            paid_fee_on_l1: Felt::from(value.paid_fee_on_l1),
            nonce: value.nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::Hash256;

    use super::*;
    use crate::chain_id::ChainId;
    use crate::rpc::transactions::ContractAddress;

    #[test]
    fn l1_handler_tx_from_message_to_l2() {
        // Test based on Goerli tx hash:
        // 0x6182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b

        let from_address = "0x000000000000000000000000be3C44c09bc1a3566F3e1CA12e5AbA0fA4Ca72Be";
        let to_address = "0x039dc79e64f4bb3289240f88e0bae7d21735bef0d1a51b2bf3c4730cb16983e1";
        let selector = "0x02f15cff7b0eed8b9beb162696cf4e3e0e35fa7032af69cd1b7d2ac67a13f40f";
        let nonce = 783082_u128;
        let fee = 30000_u128;

        let payload: Vec<Felt> = vec![1.into(), 2.into()];

        let calldata: Vec<Felt> =
            vec![Felt::from_prefixed_hex_str(from_address).unwrap(), 1.into(), 2.into()];

        let message = MessageToL2 {
            l1_contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(from_address).unwrap(),
            )
            .unwrap(),
            l2_contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(to_address).unwrap(),
            )
            .unwrap(),
            entry_point_selector: Felt::from_prefixed_hex_str(selector).unwrap(),
            payload,
            nonce: nonce.into(),
            paid_fee_on_l1: fee.into(),
        };

        let chain_id = ChainId::goerli_legacy_id();

        let transaction_hash = Felt::from_prefixed_hex_str(
            "0x6182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b",
        )
        .unwrap();

        // message hash string taken from:
        //  https://testnet.starkscan.co/tx/0x06182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b#messagelogs

        assert_eq!(
            Hash256::from_hex("0x9e658ca0f2727a3b43d0ed8171321f8b85685f5085ca5b16514d5bcb7c8a7590")
                .unwrap(),
            message.hash().unwrap()
        );

        let expected_tx = L1HandlerTransaction {
            contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(to_address).unwrap(),
            )
            .unwrap(),
            entry_point_selector: Felt::from_prefixed_hex_str(selector).unwrap(),
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            ..Default::default()
        };

        let transaction = L1HandlerTransaction::try_from_message_to_l2(message).unwrap();

        assert_eq!(transaction, expected_tx);
        assert_eq!(transaction.compute_hash(chain_id), transaction_hash);
    }
}
