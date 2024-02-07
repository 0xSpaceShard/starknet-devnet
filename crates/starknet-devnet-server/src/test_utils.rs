pub mod exported_test_utils {
    pub fn deploy_account_str() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/deploy_account.json"
        ))
        .unwrap()
    }

    pub fn declare_v1_str() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap()
    }
}
