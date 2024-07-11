use num_bigint::BigUint;
use starknet_rs_crypto::Felt;

/// Returns (high, low)
pub fn split_biguint(biguint: BigUint) -> (Felt, Felt) {
    let high = Felt::from(biguint >> 128);
    let low_mask = (BigUint::from(1_u8) << 128) - 1_u8;
    let low = Felt::from(biguint & low_mask);
    (high, low)
}

pub type Nonce = Felt;
pub type TransactionVersion = Felt;
pub type TransactionSignature = Vec<Felt>;
pub type CompiledClassHash = Felt;
pub type EntryPointSelector = Felt;
pub type Calldata = Vec<Felt>;
pub type ContractAddressSalt = Felt;
pub type BlockHash = Felt;
pub type TransactionHash = Felt;
pub type ClassHash = Felt;
pub type Key = Felt;
