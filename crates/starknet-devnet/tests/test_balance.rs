pub mod common;

mod balance_tests {
    use starknet_rs_core::types::FieldElement;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
    };
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn getting_balance_of_predeployed_contract() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_result = devnet.get_balance(&contract_address, FeeUnit::WEI).await.unwrap();

        let expected_hex_balance = format!("0x{PREDEPLOYED_ACCOUNT_INITIAL_BALANCE:x}");
        let expected_balance = FieldElement::from_hex_be(expected_hex_balance.as_str()).unwrap();
        assert_eq!(retrieved_result, expected_balance);
    }

    #[tokio::test]
    /// Tests the same logic that is used by BackgroundDevnet::get_balance
    async fn assert_balance_endpoint_response() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        for (address, expected_balance) in [
            ("0x123", "0"), // dummy address expected to have no balance
            (PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE.to_string().as_str()),
        ] {
            for unit in ["WEI", "FRI"] {
                let params = format!("address={}&unit={}", address, unit);
                let resp = devnet.get("/account_balance", Some(params)).await.unwrap();
                let json_resp = get_json_body(resp).await;

                assert_eq!(json_resp["unit"], unit);
                assert_eq!(json_resp["amount"], expected_balance);
            }
        }
    }
}
