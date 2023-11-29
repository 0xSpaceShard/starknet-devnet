use std::fmt::Display;

use starknet_rs_core::chain_id::{MAINNET, TESTNET, TESTNET2};
use starknet_rs_ff::FieldElement;

use crate::felt::Felt;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ChainId {
    #[clap(name = "MAINNET")]
    Mainnet,
    #[clap(name = "TESTNET")]
    Testnet,
    #[clap(name = "TESTNET2")]
    Testnet2,
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
            ChainId::Testnet2 => write!(f, "SN_GOERLI2"),
        }
    }
}

impl From<ChainId> for FieldElement {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => TESTNET,
            ChainId::Testnet2 => TESTNET2,
        }
    }
}

impl From<&ChainId> for FieldElement {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => TESTNET,
            ChainId::Testnet2 => TESTNET2,
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
    use super::ChainId;
    use crate::traits::ToHexString;

    #[test]
    fn check_conversion_to_starknet_api() {
        let t = ChainId::Testnet;
        let sat: starknet_api::core::ChainId = t.into();

        assert_eq!(t.to_felt().to_prefixed_hex_str(), sat.as_hex());
    }
}
