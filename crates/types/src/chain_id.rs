use std::fmt::Display;

use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_rs_core::chain_id::{MAINNET, TESTNET};
use starknet_rs_ff::FieldElement;

use crate::felt::Felt;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ChainId {
    #[clap(name = "MAINNET")]
    Mainnet,
    #[clap(name = "TESTNET")]
    Testnet,
}

impl ChainId {
    pub fn to_felt(&self) -> Felt {
        FieldElement::from(self).into()
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainId::Mainnet => write!(f, "SN_MAIN"),
            ChainId::Testnet => write!(f, "SN_GOERLI"),
        }
    }
}

impl From<ChainId> for FieldElement {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => TESTNET,
        }
    }
}

impl From<&ChainId> for FieldElement {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => TESTNET,
        }
    }
}

impl From<ChainId> for StarknetChainId {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => StarknetChainId::MainNet,
            ChainId::Testnet => StarknetChainId::TestNet,
        }
    }
}

impl From<ChainId> for starknet_api::core::ChainId {
    fn from(value: ChainId) -> Self {
        starknet_api::core::ChainId(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use cairo_felt::Felt252;
    use starknet_in_rust::definitions::block_context::StarknetChainId;

    use super::ChainId;

    #[test]
    fn check_conversion_to_starknet_in_rust_and_starknet_api() {
        let t = ChainId::Testnet;
        let st: StarknetChainId = t.into();
        let sat: starknet_api::core::ChainId = t.into();

        assert_eq!(st.to_felt(), Felt252::from_bytes_be(sat.to_string().as_bytes()));
    }
}
