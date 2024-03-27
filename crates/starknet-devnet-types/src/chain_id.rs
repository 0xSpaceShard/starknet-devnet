use std::fmt::Display;

use starknet_rs_core::chain_id::{MAINNET, TESTNET};
use starknet_rs_core::utils::parse_cairo_short_string;
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
        let felt = FieldElement::from(self);
        let str = parse_cairo_short_string(&felt).map_err(|_| std::fmt::Error)?;
        f.write_str(&str)
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

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ChainId::Mainnet), "SN_MAIN");
        assert_eq!(format!("{}", ChainId::Testnet), "SN_SEPOLIA"); // TODO failing until starknet-rs updated
    }
}
