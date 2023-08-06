use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix,
};
use starknet_in_rust::felt::Felt252;
use starknet_in_rust::transaction::{Declare, verify_version};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct DeclareTransactionV1 {
    pub(crate) inner: Declare,
    pub sender_address: ContractAddress,
    pub max_fee: u128,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub version: Felt,
    pub contract_class: ContractClass,
    pub class_hash: ClassHash,
    pub transaction_hash: TransactionHash,
    pub chain_id: Felt,
}

impl PartialEq for DeclareTransactionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.inner.sender_address == other.inner.sender_address 
        && self.inner.validate_entry_point_selector == other.inner.validate_entry_point_selector
        && self.max_fee == other.max_fee 
        && self.signature == other.signature 
        && self.nonce == other.nonce 
        && self.version == other.version 
        && self.class_hash == other.class_hash 
        && self.transaction_hash == other.transaction_hash 
        && self.chain_id == other.chain_id
    }
}

impl Eq for DeclareTransactionV1 {}

impl DeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        contract_class: ContractClass,
        chain_id: Felt,
    ) -> Result<Self> {
        if max_fee == 0 {
            return Err(Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(
                    "For declare transaction version 1, max fee cannot be 0".to_string(),
                ),
            ));
        }

        let class_hash = contract_class.generate_hash()?;
        let version = Felt::from(1);

        let mut inner = Declare {
            class_hash: class_hash.into(),
            sender_address: sender_address.try_into()?,
            tx_type: starknet_in_rust::definitions::transaction_type::TransactionType::Declare,
            validate_entry_point_selector:
                starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR.clone(),
            version: version.into(),
            max_fee: max_fee,
            signature: signature.iter().map(|felt| felt.into()).collect(),
            nonce: nonce.into(),
            hash_value: Felt252::default(),
            contract_class: contract_class.clone().try_into()?,
            skip_execute: false,
            skip_fee_transfer: false,
            skip_validate: false,
        };

        verify_version(&inner.version, inner.max_fee, &inner.nonce, &inner.signature)?;

        let (calldata, additional_data) = (vec![class_hash.into()], vec![inner.nonce.clone()]);

        let transaction_hash = calculate_transaction_hash_common(
            TransactionHashPrefix::Declare,
            inner.version.clone(),
            &inner.sender_address,
            Felt252::from(0),
            &calldata,
            max_fee,
            chain_id.into(),
            &additional_data,
        )
        .map_err(|err| {
            starknet_types::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::Syscall(
                    starknet_in_rust::syscalls::syscall_handler_errors::SyscallHandlerError::HashError(err)
                ),
            )
        })?;

        inner.hash_value = transaction_hash.clone();

        Ok(Self {
            inner,
            sender_address,
            max_fee,
            signature,
            nonce,
            version,
            contract_class,
            class_hash: class_hash,
            transaction_hash: transaction_hash.into(),
            chain_id,
        })
    }

    pub fn sender_address(&self) -> &ContractAddress {
        &self.sender_address
    }

    pub fn class_hash(&self) -> &ClassHash {
        &self.class_hash
    }
}

impl HashProducer for DeclareTransactionV1 {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(self.transaction_hash)
    }
}

#[cfg(test)]
mod tests {

    use serde::Deserialize;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_types::{contract_class::ContractClass, traits::{HashProducer, ToHexString}, felt::Felt, contract_address::ContractAddress};

    use crate::utils::test_utils::{
        dummy_cairo_0_contract_class, dummy_contract_address, dummy_felt, get_transaction_from_feeder_gateway,
    };

    #[derive(Deserialize)]
    struct FeederGatewayDeclareTransactionV1 {
        transaction_hash: Felt,
        max_fee: Felt,
        nonce: Felt,
        class_hash: Felt,
        sender_address: Felt,
    }

    #[test]
    /// test_artifact is taken from starknet-rs. https://github.com/xJonathanLEI/starknet-rs/blob/starknet-core/v0.5.1/starknet-core/test-data/contracts/cairo0/artifacts/event_example.txt
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_str = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test_artifacts/events_cairo0.txt")).unwrap();
        let cairo0 = ContractClass::cairo_0_from_json_str(&json_str).unwrap();

        // this is declare v1 transaction send with starknet-rs
        let feeder_gateway_transaction =
            get_transaction_from_feeder_gateway::<FeederGatewayDeclareTransactionV1>(
                "0x04f3480733852ec616431fd89a5e3127b49cef0ac7a71440ebdec40b1322ca9d",
            );

        assert_eq!(feeder_gateway_transaction.class_hash, cairo0.generate_hash().unwrap());

        let declare_transaction = super::DeclareTransactionV1::new(
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            u128::from_str_radix(&feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(), 16).unwrap(),
            vec![],
            feeder_gateway_transaction.nonce,
            cairo0,
            StarknetChainId::TestNet.to_felt().into(),
        ).unwrap();

        let declare_transaction_hash = declare_transaction.generate_hash().unwrap();
        assert_eq!(feeder_gateway_transaction.transaction_hash, declare_transaction_hash);
    }

    #[test]
    fn declare_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let result = super::DeclareTransactionV1::new(
            dummy_contract_address(),
            0,
            vec![],
            dummy_felt(),
            dummy_cairo_0_contract_class(),
            dummy_felt(),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For declare transaction version 1, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }
}
