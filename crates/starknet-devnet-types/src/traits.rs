use starknet_types_core::felt::Felt;

pub trait TryHashProducer {
    type Error;
    fn try_generate_hash(&self) -> Result<Felt, Self::Error>;
}

pub trait HashProducer {
    fn generate_hash(&self) -> Felt;
}