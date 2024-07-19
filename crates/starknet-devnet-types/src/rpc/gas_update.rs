use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasUpdate {
    pub gas_price_wei: NonZeroU128,
    pub data_gas_price_wei: NonZeroU128,
    pub gas_price_strk: NonZeroU128,
    pub data_gas_price_strk: NonZeroU128,
    pub generate_block: Option<bool>,
}
