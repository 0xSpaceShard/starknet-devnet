#![allow(clippy::unwrap_used)]

// TODO removable
pub fn deploy_account_str() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/rpc/deploy_account.json"
    ))
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

pub const EXPECTED_INVALID_BLOCK_ID_MSG: &str = "Invalid block ID. Expected object with key \
                                                 (block_hash or block_number) or tag \
                                                 ('pre_confirmed' or 'latest' or 'l1_accepted').";
