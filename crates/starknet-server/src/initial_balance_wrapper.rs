use std::fmt::Display;
use std::str::FromStr;

use starknet_core::constants::DEVNET_DEFAULT_INITIAL_BALANCE;
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;
use starknet_types::traits::ToDecimalString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitialBalanceWrapper(pub Felt);

impl FromStr for InitialBalanceWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let felt: Felt = BigUint::from_str(s)?.try_into()?;
        Ok(Self(felt))
    }
}

impl Display for InitialBalanceWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_decimal_string())
    }
}

impl Default for InitialBalanceWrapper {
    fn default() -> Self {
        Self(Felt::from(DEVNET_DEFAULT_INITIAL_BALANCE))
    }
}
