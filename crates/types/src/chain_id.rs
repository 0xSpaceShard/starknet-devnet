use std::fmt::Display;

use starknet_in_rust::definitions::block_context::StarknetChainId;
use starknet_rs_core::chain_id::{MAINNET, TESTNET, TESTNET2};
use starknet_rs_ff::FieldElement;

use crate::felt::Felt;

#[derive(Clone, Copy, Debug)]
pub enum ChainId {
    MainNet,
    TestNet,
    TestNet2,
}

impl ChainId {
    pub fn to_felt(&self) -> Felt {
        FieldElement::from(self).into()
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainId::MainNet => write!(f, "SN_MAIN"),
            ChainId::TestNet => write!(f, "SN_GOERLI"),
            ChainId::TestNet2 => write!(f, "SN_GOERLI2"),
        }
    }
}

impl From<ChainId> for FieldElement {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::MainNet => MAINNET,
            ChainId::TestNet => TESTNET,
            ChainId::TestNet2 => TESTNET2,
        }
    }
}

impl From<&ChainId> for FieldElement {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::MainNet => MAINNET,
            ChainId::TestNet => TESTNET,
            ChainId::TestNet2 => TESTNET2,
        }
    }
}

impl From<ChainId> for StarknetChainId {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::MainNet => StarknetChainId::MainNet,
            ChainId::TestNet => StarknetChainId::TestNet,
            ChainId::TestNet2 => StarknetChainId::TestNet2,
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
        let t = ChainId::TestNet;
        let st: StarknetChainId = t.into();
        let sat: starknet_api::core::ChainId = t.into();

        assert_eq!(st.to_felt(), Felt252::from_bytes_be(sat.to_string().as_bytes()));
    }
}
