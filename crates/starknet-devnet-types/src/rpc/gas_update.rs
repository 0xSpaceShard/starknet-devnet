use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasUpdateRequest {
    pub gas_price_wei: Option<NonZeroU128>,
    pub data_gas_price_wei: Option<NonZeroU128>,
    pub gas_price_strk: Option<NonZeroU128>,
    pub data_gas_price_strk: Option<NonZeroU128>,
    pub generate_block: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasUpdate {
    pub gas_price_wei: NonZeroU128,
    pub data_gas_price_wei: NonZeroU128,
    pub gas_price_strk: NonZeroU128,
    pub data_gas_price_strk: NonZeroU128,
}

impl GasUpdate {
    pub fn update(&mut self, request: GasUpdateRequest) {
        if let Some(gas_price_wei) = request.gas_price_wei {
            self.gas_price_wei = gas_price_wei;
        }
        if let Some(data_gas_price_wei) = request.data_gas_price_wei {
            self.data_gas_price_wei = data_gas_price_wei;
        }
        if let Some(gas_price_strk) = request.gas_price_strk {
            self.gas_price_strk = gas_price_strk;
        }
        if let Some(data_gas_price_strk) = request.data_gas_price_strk {
            self.data_gas_price_strk = data_gas_price_strk;
        }
    }
}
