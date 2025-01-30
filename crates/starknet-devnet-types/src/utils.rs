#[cfg(test)]
pub(crate) mod test_utils {
    use starknet_api::{
        data_availability::DataAvailabilityMode, transaction::fields::ResourceBounds,
    };

    use crate::rpc::transactions::ResourceBoundsWrapper;

    pub(crate) const CAIRO_0_RPC_CONTRACT_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/cairo_0_rpc.json");

    /// contract declared in transaction https://alpha4.starknet.io/feeder_gateway/get_transaction?transactionHash=0x01b852f1fe2b13db21a44f8884bc4b7760dc277bb3820b970dba929860275617
    /// cairo code is in the same directory as the sierra artifacts
    pub(crate) const CAIRO_1_EVENTS_CONTRACT_PATH: &str =
        "../../contracts/test_artifacts/cairo1/events/events_2.0.1_compiler.sierra";

    pub(crate) const CAIRO_1_CONTRACT_SIERRA_HASH: &str =
        "0x113bf26d112a164297e04381212c9bd7409f07591f0a04f539bdf56693eaaf3";

    /// Converts integer to DataAvailabilityMode
    /// # Arguments
    ///
    /// * `da_mode` - integer representing the data availability mode
    pub(crate) fn from_u8_to_da_mode(da_mode: u8) -> DataAvailabilityMode {
        match da_mode {
            0 => DataAvailabilityMode::L1,
            1 => DataAvailabilityMode::L2,
            _ => panic!("Invalid data availability mode"),
        }
    }

    pub(crate) fn convert_from_sn_api_l1_resource_bounds(
        l1_resource_bounds: ResourceBounds,
    ) -> ResourceBoundsWrapper {
        ResourceBoundsWrapper::new(
            l1_resource_bounds.max_amount.0,
            l1_resource_bounds.max_price_per_unit.0,
            0,
            0,
        )
    }
}
