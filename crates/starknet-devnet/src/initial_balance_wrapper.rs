use std::fmt::Display;
use std::str::FromStr;

use starknet_core::constants::DEVNET_DEFAULT_INITIAL_BALANCE;
use starknet_types::rpc::state::Balance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitialBalanceWrapper(pub Balance);

impl FromStr for InitialBalanceWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let balance = Balance::from_str(s)?;
        Ok(Self(balance))
    }
}

impl Display for InitialBalanceWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for InitialBalanceWrapper {
    fn default() -> Self {
        Self(Balance::from(DEVNET_DEFAULT_INITIAL_BALANCE))
    }
}
