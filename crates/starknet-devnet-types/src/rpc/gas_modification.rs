use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasModificationRequest {
    pub gas_price_wei: Option<NonZeroU128>,
    pub data_gas_price_wei: Option<NonZeroU128>,
    pub gas_price_fri: Option<NonZeroU128>,
    pub data_gas_price_fri: Option<NonZeroU128>,
    pub generate_block: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct GasModification {
    pub gas_price_wei: NonZeroU128,
    pub data_gas_price_wei: NonZeroU128,
    pub gas_price_fri: NonZeroU128,
    pub data_gas_price_fri: NonZeroU128,
}

impl GasModification {
    pub fn update(&mut self, request: GasModificationRequest) {
        if let Some(gas_price_wei) = request.gas_price_wei {
            self.gas_price_wei = gas_price_wei;
        }
        if let Some(data_gas_price_wei) = request.data_gas_price_wei {
            self.data_gas_price_wei = data_gas_price_wei;
        }
        if let Some(gas_price_fri) = request.gas_price_fri {
            self.gas_price_fri = gas_price_fri;
        }
        if let Some(data_gas_price_fri) = request.data_gas_price_fri {
            self.data_gas_price_fri = data_gas_price_fri;
        }
    }
}
