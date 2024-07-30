fn extract_overall_fee(simulation_result: &serde_json::Value) -> u128 {
    let fee_hex = simulation_result["fee_estimation"]["overall_fee"].as_str().unwrap();
    let fee_hex_stripped = fee_hex.strip_prefix("0x").unwrap();
    u128::from_str_radix(fee_hex_stripped, 16).unwrap()
}

/// Calculating the fee of a transaction depends on fee weights of vm resources and the resource
/// usage. The fee from vm usage is the heaviest product of cairo resource usage multiplied
/// by cairo resource fee cost, rounded down to the nearest integer. There is a
/// possibility that although some of the resources present in the resp_no_flags are not in
/// resp_skip_validation, this doesnt mean that the fee will be higher, for example:
/// the fee from vm usage is determined from the `steps` and the transaction that skips
/// validation might have less steps, but due to the rounding if the multiplied product
/// falls in the range (7,8] both will be rounded to 7
pub fn assert_fee_in_resp_at_least_equal(
    resp_no_flags: &serde_json::Value,
    resp_skip_validation: &serde_json::Value,
) {
    let no_flags_fee = extract_overall_fee(resp_no_flags);
    let skip_validation_fee = extract_overall_fee(resp_skip_validation);
    assert!(no_flags_fee.ge(&skip_validation_fee));
}

pub fn assert_difference_if_validation(
    resp_no_flags: &serde_json::Value,
    resp_skip_validation: &serde_json::Value,
    expected_contract_address: &str,
    should_skip_fee_invocation: bool,
) {
    let no_flags_trace = &resp_no_flags["transaction_trace"];
    assert_eq!(
        no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
        expected_contract_address
    );
    assert!(no_flags_trace["state_diff"].as_object().is_some());

    let skip_validation_trace = &resp_skip_validation["transaction_trace"];
    assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
    assert!(skip_validation_trace["state_diff"].as_object().is_some());

    assert_eq!(
        skip_validation_trace["fee_transfer_invocation"].as_object().is_none(),
        should_skip_fee_invocation
    );
    assert_eq!(
        no_flags_trace["fee_transfer_invocation"].as_object().is_none(),
        should_skip_fee_invocation
    );

    assert!(
        no_flags_trace["execution_resources"]["steps"].as_u64().unwrap()
            > skip_validation_trace["execution_resources"]["steps"].as_u64().unwrap()
    );

    assert_fee_in_resp_at_least_equal(resp_no_flags, resp_skip_validation);
}
