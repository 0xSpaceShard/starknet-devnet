use std::fmt::Display;
use std::str::FromStr;

use starknet_rs_core::chain_id::{MAINNET, SEPOLIA, TESTNET};
use starknet_rs_core::utils::{cairo_short_string_to_felt, parse_cairo_short_string};
use starknet_rs_ff::FieldElement;

use crate::error::ConversionError;
use crate::felt::Felt;

#[derive(Clone, Copy, Debug)]
pub enum ChainId {
    Mainnet,
    Testnet,
    Custom(FieldElement),
}

impl ChainId {
    pub fn goerli_legacy_id() -> Felt {
        TESTNET.into()
    }

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

impl FromStr for ChainId {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uppercase_chain_id_str = s.to_ascii_uppercase();
        let chain_id = match uppercase_chain_id_str.as_str() {
            "MAINNET" => ChainId::Mainnet,
            "TESTNET" => ChainId::Testnet,
            _ => {
                let felt = cairo_short_string_to_felt(&uppercase_chain_id_str)
                    .map_err(|err| ConversionError::OutOfRangeError(err.to_string()))?;
                ChainId::Custom(felt)
            }
        };

        Ok(chain_id)
    }
}

impl From<ChainId> for FieldElement {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => SEPOLIA,
            ChainId::Custom(felt) => felt,
        }
    }
}

impl From<&ChainId> for FieldElement {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => SEPOLIA,
            ChainId::Custom(felt) => *felt,
        }
    }
}

impl From<FieldElement> for ChainId {
    fn from(value: FieldElement) -> Self {
        if value == MAINNET {
            ChainId::Mainnet
        } else if value == SEPOLIA {
            ChainId::Testnet
        } else {
            ChainId::Custom(value)
        }
    }
}

impl From<ChainId> for starknet_api::core::ChainId {
    fn from(value: ChainId) -> Self {
        starknet_api::core::ChainId(value.to_string())
    }
}

impl From<starknet_api::core::ChainId> for ChainId {
    fn from(value: starknet_api::core::ChainId) -> Self {
        match value.0.as_str() {
            "MAINNET" => ChainId::Mainnet,
            "SEPOLIA" => ChainId::Testnet,
            _ => ChainId::Testnet,
        }
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
        assert_eq!(format!("{}", ChainId::Testnet), "SN_SEPOLIA");
    }
}
