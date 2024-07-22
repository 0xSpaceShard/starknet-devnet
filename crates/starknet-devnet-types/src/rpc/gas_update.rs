use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasUpdate {
    pub gas_price_wei: Option<NonZeroU128>,
    pub data_gas_price_wei: Option<NonZeroU128>,
    pub gas_price_strk: Option<NonZeroU128>,
    pub data_gas_price_strk: Option<NonZeroU128>,
    pub generate_block: Option<bool>,
}

impl GasUpdate {
    pub fn is_any_field_set(&self) -> bool {
        self.gas_price_wei.is_some()
            || self.data_gas_price_wei.is_some()
            || self.gas_price_strk.is_some()
            || self.data_gas_price_strk.is_some()
            || self.generate_block.is_some()
    }
}
