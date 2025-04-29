use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasModificationRequest {
    pub gas_price_wei: Option<NonZeroU128>,
    pub data_gas_price_wei: Option<NonZeroU128>,
    pub gas_price_fri: Option<NonZeroU128>,
    pub data_gas_price_fri: Option<NonZeroU128>,
    pub l2_gas_price_wei: Option<NonZeroU128>,
    pub l2_gas_price_fri: Option<NonZeroU128>,
    pub generate_block: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct GasModification {
    pub l1_gas_price: NonZeroU128,
    pub l1_data_gas_price: NonZeroU128,
    pub l2_gas_price: NonZeroU128,
}

impl GasModification {
    pub fn update(&mut self, request: GasModificationRequest) {
        if let Some(gas_price_fri) = request.gas_price_fri {
            self.l1_gas_price = gas_price_fri;
        }
        if let Some(data_gas_price_fri) = request.data_gas_price_fri {
            self.l1_data_gas_price = data_gas_price_fri;
        }
        if let Some(l2_gas_price_fri) = request.l2_gas_price_fri {
            self.l2_gas_price = l2_gas_price_fri;
        }
    }
}

// TODO impl convertor to GasPriceVector
