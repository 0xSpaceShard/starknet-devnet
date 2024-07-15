use std::fmt::Display;

#[allow(deprecated)]
use starknet_rs_core::chain_id::{MAINNET, SEPOLIA, TESTNET};
use starknet_rs_core::utils::parse_cairo_short_string;
use starknet_rs_crypto::Felt;

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChainId {
    Mainnet,
    Testnet,
}

impl ChainId {
    pub fn goerli_legacy_id() -> Felt {
        #[allow(deprecated)]
        TESTNET
    }

    pub fn to_felt(&self) -> Felt {
        Felt::from(self)
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let felt = Felt::from(self);
        let str = parse_cairo_short_string(&felt).map_err(|_| std::fmt::Error)?;
        f.write_str(&str)
    }
}

impl From<ChainId> for Felt {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => SEPOLIA,
        }
    }
}

impl From<&ChainId> for Felt {
    fn from(value: &ChainId) -> Self {
        match value {
            ChainId::Mainnet => MAINNET,
            ChainId::Testnet => SEPOLIA,
        }
    }
}

impl From<ChainId> for starknet_api::core::ChainId {
    fn from(value: ChainId) -> Self {
        match value {
            ChainId::Mainnet => Self::Mainnet,
            ChainId::Testnet => Self::Sepolia,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChainId;

    #[test]
    fn check_conversion_to_starknet_api() {
        let t = ChainId::Testnet;
        let sat: starknet_api::core::ChainId = t.into();

        assert_eq!(t.to_felt().to_hex_string(), sat.as_hex());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ChainId::Mainnet), "SN_MAIN");
        assert_eq!(format!("{}", ChainId::Testnet), "SN_SEPOLIA");
    }
}
