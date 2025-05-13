use std::num::NonZeroU128;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GasModificationRequest {
    pub l1_gas_price: Option<NonZeroU128>,
    pub l1_data_gas_price: Option<NonZeroU128>,
    pub l2_gas_price: Option<NonZeroU128>,
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
        if let Some(l1_gas_price) = request.l1_gas_price {
            self.l1_gas_price = l1_gas_price;
        }
        if let Some(l1_data_gas_price) = request.l1_data_gas_price {
            self.l1_data_gas_price = l1_data_gas_price;
        }
        if let Some(l2_gas_price) = request.l2_gas_price {
            self.l2_gas_price = l2_gas_price;
        }
    }
}

// TODO impl convertor to GasPriceVector
