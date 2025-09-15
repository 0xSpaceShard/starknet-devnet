/// Returns Err if `text` does not contain `pattern`
pub fn assert_contains(text: &str, pattern: &str) -> Result<(), anyhow::Error> {
    if !text.contains(pattern) {
        anyhow::bail!(
            "Failed content assertion!
    Pattern: '{pattern}'
    not present in
    Text: '{text}'"
        );
    }

    Ok(())
}

pub const EXPECTED_INVALID_BLOCK_ID_MSG: &str = "Invalid block ID. Expected object with key \
                                                 (block_hash or block_number) or tag \
                                                 ('pre_confirmed' or 'latest' or 'l1_accepted').";
