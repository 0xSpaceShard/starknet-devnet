use starknet_types_core::felt::Felt;

pub trait HashProducer {
    type Error;
    fn generate_hash(&self) -> Result<Felt, Self::Error>;
}
