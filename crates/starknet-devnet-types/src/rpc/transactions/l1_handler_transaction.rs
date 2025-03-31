use std::sync::Arc;

use serde::Serialize;
use starknet_api::core::{
    ContractAddress as ApiContractAddress, EntryPointSelector as ApiEntryPointSelector,
    Nonce as ApiNonce,
};
use starknet_api::executable_transaction::L1HandlerTransaction as ApiL1HandlerTransaction;
use starknet_api::transaction::fields::{Calldata as ApiCalldata, Fee as ApiFee};
use starknet_api::transaction::{
    TransactionHash as ApiTransactionHash, TransactionVersion as ApiTransactionVersion,
};
use starknet_rs_core::crypto::compute_hash_on_elements;
use starknet_rs_core::types::{Felt, Hash256};

use super::serialize_paid_fee_on_l1;
use crate::constants::PREFIX_L1_HANDLER;
use crate::contract_address::ContractAddress;
use crate::error::{ConversionError, DevnetResult, Error};
use crate::felt::{Calldata, EntryPointSelector, Nonce, TransactionVersion, try_felt_to_num};
use crate::rpc::messaging::MessageToL2;

#[derive(Debug, Clone, Default, Serialize, Eq, PartialEq)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct L1HandlerTransaction {
    /// The hash of the L1 transaction that triggered this L1 handler execution.
    /// Omissible if received via mock (devnet_postmanSendMessageToL2)
    pub l1_transaction_hash: Option<Hash256>,
    pub version: TransactionVersion,
    pub nonce: Nonce,
    pub contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
    #[serde(
        serialize_with = "serialize_paid_fee_on_l1",
        deserialize_with = "super::deserialize_paid_fee_on_l1"
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
        let fee = Felt::ZERO;

        compute_hash_on_elements(&[
            PREFIX_L1_HANDLER,
            self.version,
            self.contract_address.into(),
            self.entry_point_selector,
            compute_hash_on_elements(&self.calldata),
            fee,
            chain_id,
            self.nonce,
        ])
    }

    /// Creates a blockifier version of `L1HandlerTransaction`.
    pub fn create_sn_api_transaction(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<ApiL1HandlerTransaction> {
        let transaction = ApiL1HandlerTransaction {
            tx: starknet_api::transaction::L1HandlerTransaction {
                contract_address: ApiContractAddress::try_from(self.contract_address)?,
                entry_point_selector: ApiEntryPointSelector(self.entry_point_selector),
                calldata: ApiCalldata(Arc::new(self.calldata.clone())),
                nonce: ApiNonce(self.nonce),
                version: ApiTransactionVersion(self.version),
            },
            paid_fee_on_l1: ApiFee(self.paid_fee_on_l1),
            tx_hash: ApiTransactionHash(self.compute_hash(chain_id)),
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
        let paid_fee_on_l1: u128 = try_felt_to_num(message.paid_fee_on_l1).map_err(|_| {
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
            l1_transaction_hash: message.l1_transaction_hash,
            version: Felt::ZERO, // currently, only version 0 is supported
        })
    }
}

impl TryFrom<&L1HandlerTransaction> for MessageToL2 {
    type Error = Error;

    fn try_from(value: &L1HandlerTransaction) -> Result<Self, Self::Error> {
        let l1_contract_address = value.calldata.first().ok_or(Error::ConversionError(
            ConversionError::InvalidInternalStructure(
                "L1HandlerTransaction calldata is expected to have at least one element"
                    .to_string(),
            ),
        ))?;

        let payload = value.calldata[1..].to_vec();
        Ok(MessageToL2 {
            l1_transaction_hash: value.l1_transaction_hash,
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
    use crate::felt::felt_from_prefixed_hex;
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

        let payload: Vec<Felt> = vec![Felt::ONE, Felt::TWO];

        let calldata: Vec<Felt> =
            vec![felt_from_prefixed_hex(from_address).unwrap(), Felt::ONE, Felt::TWO];

        let message = MessageToL2 {
            l1_transaction_hash: None,
            l1_contract_address: ContractAddress::new(
                felt_from_prefixed_hex(from_address).unwrap(),
            )
            .unwrap(),
            l2_contract_address: ContractAddress::new(felt_from_prefixed_hex(to_address).unwrap())
                .unwrap(),
            entry_point_selector: felt_from_prefixed_hex(selector).unwrap(),
            payload,
            nonce: nonce.into(),
            paid_fee_on_l1: fee.into(),
        };

        let transaction_hash = felt_from_prefixed_hex(
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
            contract_address: ContractAddress::new(felt_from_prefixed_hex(to_address).unwrap())
                .unwrap(),
            entry_point_selector: felt_from_prefixed_hex(selector).unwrap(),
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            ..Default::default()
        };

        let transaction = L1HandlerTransaction::try_from_message_to_l2(message).unwrap();

        assert_eq!(transaction, expected_tx);
        assert_eq!(transaction.compute_hash(ChainId::goerli_legacy_id()), transaction_hash);
    }
}
