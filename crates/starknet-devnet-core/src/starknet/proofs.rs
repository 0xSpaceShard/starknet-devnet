use starknet_api::core::OsChainInfo;
use starknet_api::transaction::fields::{PROOF_VERSION, VIRTUAL_OS_OUTPUT_VERSION, VIRTUAL_SNOS};
use starknet_rs_core::types::Felt;
use starknet_types::felt::ProofFacts;
use starknet_types::proof::Proof;
use starknet_types::rpc::block::BlockId;
use starknet_types::rpc::transactions::BroadcastedInvokeTransaction;
use starknet_types_core::hash::{Pedersen, StarkHash};
use tracing::debug;

use crate::error::{DevnetResult, ProvingError};
use crate::starknet::Starknet;

static DEVNET_PROOF_MAGIC: u64 = 0xFAFAFAFA;

/// Convert a Felt to Proof using its byte representation
fn felt_to_proof(felt: Felt) -> Proof {
    Proof::new(felt.to_bytes_be().to_vec())
}

/// Convert a Proof back to Felt for verification
fn proof_to_felt(proof: &Proof) -> Option<Felt> {
    let proof_data = proof.inner();
    if proof_data.len() != 32 {
        return None;
    }

    let bytes: [u8; 32] = proof_data.as_slice().try_into().ok()?;
    Some(Felt::from_bytes_be(&bytes))
}

pub fn prove_transaction(
    starknet: &Starknet,
    block_id: BlockId,
    broadcasted_invoke_transaction: BroadcastedInvokeTransaction,
) -> DevnetResult<(Proof, ProofFacts)> {
    debug!("Generating devnet proof for invoke transaction at block_id: {block_id:?}");

    let block_context = starknet.block_context.clone();
    let program_hash = block_context
        .versioned_constants()
        .os_constants
        .allowed_virtual_os_program_hashes
        .first()
        .ok_or(ProvingError::NoVirtualProgramHashesAllowed)?;
    let block = starknet.get_block(&block_id).map_err(|_| ProvingError::InvalidBlockId)?;
    let block_number_felt = Felt::from(block.block_number().0);
    let config_hash = OsChainInfo::from(block_context.chain_info())
        .compute_virtual_os_config_hash()
        .map_err(|e| ProvingError::Other(e.to_string()))?;

    let tx_hash = broadcasted_invoke_transaction
        .create_sn_api_invoke(&starknet.chain_id().to_felt())
        .map_err(|e| ProvingError::Other(e.to_string()))?
        .tx_hash
        .0;

    debug!("Computed invoke transaction hash for proof generation: {tx_hash:#x}");

    let proof_felt = Pedersen::hash_array(&[tx_hash, DEVNET_PROOF_MAGIC.into()]);
    let proof = felt_to_proof(proof_felt);

    let last_field = Pedersen::hash_array(&[
        PROOF_VERSION,
        VIRTUAL_SNOS,
        *program_hash,
        VIRTUAL_OS_OUTPUT_VERSION,
        block_number_felt,
        block.block_hash(),
        config_hash,
        proof_felt,
    ]);

    let proof_facts = vec![
        PROOF_VERSION,
        VIRTUAL_SNOS,
        *program_hash,
        VIRTUAL_OS_OUTPUT_VERSION,
        block_number_felt,
        block.block_hash(),
        config_hash,
        last_field,
    ];

    debug!(
        "Generated proof successfully (block_number: {}, proof_len: {}, proof_facts_len: {})",
        block.block_number().0,
        proof.len(),
        proof_facts.len()
    );

    Ok((proof, proof_facts))
}

pub fn verify_proof(proof: Proof, proof_facts: ProofFacts) -> bool {
    let mut input = proof_facts.clone();
    if input.len() != 8 {
        debug!("Proof verification failed: invalid proof_facts length: {}", input.len());
        return false;
    }
    let last_field = if let Some(v) = input.pop() {
        v
    } else {
        return false;
    };

    // Convert proof from Vec<i64> back to Felt
    let proof_felt = if let Some(felt) = proof_to_felt(&proof) {
        felt
    } else {
        debug!(
            "Proof verification failed: could not convert proof to felt (proof_len: {})",
            proof.len()
        );
        return false;
    };

    input.push(proof_felt);
    let is_valid = Pedersen::hash_array(&input) == last_field;
    if is_valid {
        debug!("Proof verification succeeded ");
    } else {
        debug!("Proof verification failed: commitment mismatch");
    }
    is_valid
}

#[cfg(test)]
mod tests {
    use starknet_api::data_availability::DataAvailabilityMode;
    use starknet_api::transaction::fields::Tip;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };

    use super::*;
    use crate::starknet::starknet_config::StarknetConfig;

    fn create_test_starknet() -> Starknet {
        let config = StarknetConfig::default();
        Starknet::new(&config).expect("Failed to create Starknet instance")
    }

    fn create_test_invoke_transaction() -> BroadcastedInvokeTransaction {
        BroadcastedInvokeTransaction::V3(BroadcastedInvokeTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce: Felt::ZERO,
                resource_bounds: ResourceBoundsWrapper::new(100_000, 1, 100_000, 1, 100_000, 1),
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            },
            sender_address: ContractAddress::new(Felt::from(0x123u64))
                .expect("valid contract address"),
            calldata: vec![Felt::ONE, Felt::TWO],
            account_deployment_data: vec![],
            proof: None,
            proof_facts: None,
        })
    }

    #[test]
    fn test_prove_transaction_generates_valid_proof() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (proof, proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();

        // Verify proof facts has correct length
        assert_eq!(proof_facts.len(), 8, "proof_facts should have 8 elements");

        // Verify proof has correct length (32 u8 values)
        assert_eq!(proof.len(), 32, "proof should have 32 u8 values");
    }

    #[test]
    fn test_verify_proof_accepts_valid_proof() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (proof, proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();

        assert!(verify_proof(proof, proof_facts), "valid proof should be verified");
    }

    #[test]
    fn test_verify_proof_rejects_wrong_proof() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (_proof, proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();
        let wrong_proof = vec![0xDEu8; 32];

        assert!(!verify_proof(wrong_proof.into(), proof_facts), "wrong proof should be rejected");
    }

    #[test]
    fn test_verify_proof_rejects_modified_proof_facts() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (proof, mut proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();

        // Modify one of the proof facts
        if let Some(fact) = proof_facts.get_mut(0) {
            *fact = Felt::from(0xBADu64);
        }

        assert!(!verify_proof(proof, proof_facts), "proof with modified facts should be rejected");
    }

    #[test]
    fn test_verify_proof_rejects_wrong_length_proof_facts() {
        let proof = vec![0x12u8; 32];

        // Too few elements
        let short_proof_facts = vec![Felt::ONE, Felt::TWO, Felt::THREE];
        assert!(
            !verify_proof(proof.clone().into(), short_proof_facts),
            "short proof_facts should be rejected"
        );

        // Too many elements
        let long_proof_facts = vec![Felt::ONE; 10];
        assert!(
            !verify_proof(proof.into(), long_proof_facts),
            "long proof_facts should be rejected"
        );
    }

    #[test]
    fn test_verify_proof_rejects_empty_proof_facts() {
        let proof = vec![0x12u8; 32];
        let empty_proof_facts = vec![];

        assert!(
            !verify_proof(proof.into(), empty_proof_facts),
            "empty proof_facts should be rejected"
        );
    }

    #[test]
    fn test_verify_proof_rejects_wrong_length_proof() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (_proof, proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();

        // Proof with wrong length (too few elements)
        let short_proof = vec![0x12u8; 2];
        assert!(
            !verify_proof(short_proof.into(), proof_facts.clone()),
            "short proof should be rejected"
        );

        // Proof with wrong length (too many elements)
        let long_proof = vec![0x12u8; 40];
        assert!(!verify_proof(long_proof.into(), proof_facts), "long proof should be rejected");
    }

    #[test]
    fn test_prove_transaction_deterministic() {
        let starknet = create_test_starknet();
        let tx1 = create_test_invoke_transaction();
        let tx2 = create_test_invoke_transaction();

        let (proof1, proof_facts1) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx1,
        )
        .unwrap();
        let (proof2, proof_facts2) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx2,
        )
        .unwrap();

        // Same transactions should produce same proofs
        assert_eq!(proof1, proof2, "same transactions should produce same proof");
        assert_eq!(proof_facts1, proof_facts2, "same transactions should produce same proof_facts");
    }

    #[test]
    fn test_prove_transaction_different_for_different_transactions() {
        let starknet = create_test_starknet();
        let tx1 = create_test_invoke_transaction();
        let mut tx2 = create_test_invoke_transaction();

        // Modify tx2 to be different
        let BroadcastedInvokeTransaction::V3(ref mut v3) = tx2;
        v3.common.nonce = Felt::ONE;

        let (proof1, _) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx1,
        )
        .unwrap();
        let (proof2, _) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx2,
        )
        .unwrap();

        // Different transactions should produce different proofs
        assert_ne!(proof1, proof2, "different transactions should produce different proofs");
    }

    #[test]
    fn test_proof_facts_structure() {
        let starknet = create_test_starknet();
        let tx = create_test_invoke_transaction();

        let (proof, proof_facts) = prove_transaction(
            &starknet,
            BlockId::Tag(starknet_types::rpc::block::BlockTag::Latest),
            tx,
        )
        .unwrap();

        // Verify proof_facts contains expected fields
        assert_eq!(proof_facts[0], PROOF_VERSION, "first field should be proof_version");
        assert_eq!(proof_facts[1], VIRTUAL_SNOS, "second field should be variant_marker");

        // Last field should be hash of all previous fields plus proof_felt
        let proof_felt = proof_to_felt(&proof).expect("proof should convert to felt");
        let mut input = proof_facts[0..7].to_vec();
        input.push(proof_felt);
        let expected_last_field = Pedersen::hash_array(&input);
        assert_eq!(
            proof_facts[7], expected_last_field,
            "last field should be hash of previous fields plus proof"
        );
    }
}
