#![allow(clippy::unwrap_used)]
pub fn deploy_account_str() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/rpc/deploy_account.json"
    ))
    .unwrap()
}

pub fn declare_v1_str() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/declare_v1.json"))
        .unwrap()
}

/// Panics if `text` does not contain `pattern`
pub fn assert_contains(text: &str, pattern: &str) {
    if !text.contains(pattern) {
        panic!(
            "Failed content assertion!
    Pattern: '{pattern}'
    not present in
    Text: '{text}'"
        );
    }
}
