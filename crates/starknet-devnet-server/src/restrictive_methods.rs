use std::collections::HashMap;

use lazy_static::lazy_static;
lazy_static! {
    static ref MAPPING_HTTP_URIS_TO_RPC_METHODS: HashMap<&'static str, &'static str> =
        HashMap::from([
            ("/dump", "devnet_dump"),
            ("/load", "devnet_load"),
            ("/set_time", "devnet_setTime"),
            ("/increase_time", "devnet_increaseTime"),
            ("/create_block", "devnet_createBlock"),
            ("/abort_blocks", "devnet_abortBlocks"),
            ("/restart", "devnet_restart"),
            ("/mint", "devnet_mint"),
            ("/postman/load_l1_messaging_contract", "devnet_postmanLoad"),
            ("/postman/flush", "devnet_postmanFlush"),
            ("/postman/send_message_to_l2", "devnet_postmanSendMessageToL2"),
            ("/postman/consume_message_from_l2", "devnet_postmanConsumeMessageFromL2"),
            ("/predeployed_accounts", "devnet_getPredeployedAccounts"),
            ("/account_balance", "devnet_getAccountBalance"),
            ("/config", "devnet_getConfig"),
        ]);
    static ref MAPPING_RPC_METHODS_TO_HTTP_URIS: HashMap<&'static str, &'static str> =
        MAPPING_HTTP_URIS_TO_RPC_METHODS.iter().map(|(k, v)| (*v, *k)).collect();
    pub static ref DEFAULT_RESTRICTED_JSON_RPC_METHODS: Vec<&'static str> = vec![
        "devnet_mint",
        "devnet_restart",
        "devnet_createBlock",
        "devnet_abortBlocks",
        "devnet_impersonateAccount",
        "devnet_autoImpersonate"
    ];
    static ref DEFAULT_RESTRICTED_HTTP_URIS: Vec<&'static str> =
        DEFAULT_RESTRICTED_JSON_RPC_METHODS
            .iter()
            .filter_map(|method| MAPPING_RPC_METHODS_TO_HTTP_URIS.get(method))
            .copied()
            .collect();
}

pub(crate) fn is_json_rpc_method_restricted(
    json_rpc_method: &str,
    restricted_methods: &[&str],
) -> bool {
    if restricted_methods.contains(&json_rpc_method) {
        return true;
    }

    match MAPPING_RPC_METHODS_TO_HTTP_URIS.get(json_rpc_method) {
        Some(http_uri) => restricted_methods.contains(http_uri),
        None => false,
    }
}

pub(crate) fn is_uri_path_restricted(uri_path: &str, restricted_uris: &[&str]) -> bool {
    if restricted_uris.contains(&uri_path) {
        return true;
    }

    match MAPPING_HTTP_URIS_TO_RPC_METHODS.get(uri_path) {
        Some(json_rpc_method) => restricted_uris.contains(json_rpc_method),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_RESTRICTED_HTTP_URIS, DEFAULT_RESTRICTED_JSON_RPC_METHODS};
    use crate::restrictive_methods::{is_json_rpc_method_restricted, is_uri_path_restricted};

    #[test]
    fn test_mappings_length_because_json_rpc_methods_are_greater_than_http_methods() {
        assert!(
            crate::restrictive_methods::DEFAULT_RESTRICTED_JSON_RPC_METHODS.len()
                > crate::restrictive_methods::DEFAULT_RESTRICTED_HTTP_URIS.len()
        )
    }
    #[test]
    fn test_provided_method_is_restricted() {
        assert_is_restricted("devnet_mint", DEFAULT_RESTRICTED_HTTP_URIS.as_slice());
        assert_is_restricted("/mint", DEFAULT_RESTRICTED_HTTP_URIS.as_slice());
        assert_is_restricted("devnet_mint", DEFAULT_RESTRICTED_JSON_RPC_METHODS.as_slice());
        assert_is_restricted("/mint", DEFAULT_RESTRICTED_JSON_RPC_METHODS.as_slice());
        assert_is_restricted(
            "devnet_impersonateAccount",
            DEFAULT_RESTRICTED_JSON_RPC_METHODS.as_slice(),
        );
        assert_is_restricted("devnet_mint", &["/mint", "dump", "devnet_mint"]);
    }

    #[test]
    fn test_provided_method_is_not_restricted() {
        assert_is_not_restricted(
            "devnet_impersonateAccount",
            DEFAULT_RESTRICTED_HTTP_URIS.as_slice(),
        );
        assert_is_not_restricted("devnet_config", DEFAULT_RESTRICTED_JSON_RPC_METHODS.as_slice());

        assert_is_not_restricted("devnet_config", DEFAULT_RESTRICTED_HTTP_URIS.as_slice());
        assert_is_not_restricted("/config", DEFAULT_RESTRICTED_JSON_RPC_METHODS.as_slice());
        assert_is_not_restricted("/config", DEFAULT_RESTRICTED_HTTP_URIS.as_slice());
    }

    fn assert_is_restricted(method: &str, restricted_methods: &[&str]) {
        if method.contains('/') {
            assert!(is_uri_path_restricted(method, restricted_methods));
        } else {
            assert!(is_json_rpc_method_restricted(method, restricted_methods));
        }
    }
    fn assert_is_not_restricted(method: &str, restricted_methods: &[&str]) {
        if method.contains('/') {
            assert!(!is_uri_path_restricted(method, restricted_methods));
        } else {
            assert!(!is_json_rpc_method_restricted(method, restricted_methods));
        }
    }
}
