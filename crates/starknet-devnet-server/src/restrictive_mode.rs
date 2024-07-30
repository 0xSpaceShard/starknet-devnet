use std::collections::HashMap;

use lazy_static::lazy_static;
lazy_static! {
    static ref HTTP_URI_TO_RPC_METHOD: HashMap<&'static str, String> = HashMap::from([
        ("/dump", "devnet_dump".into()),
        ("/load", "devnet_load".into()),
        ("/set_time", "devnet_setTime".into()),
        ("/increase_time", "devnet_increaseTime".into()),
        ("/create_block", "devnet_createBlock".into()),
        ("/abort_blocks", "devnet_abortBlocks".into()),
        ("/restart", "devnet_restart".into()),
        ("/mint", "devnet_mint".into()),
        ("/postman/load_l1_messaging_contract", "devnet_postmanLoad".into()),
        ("/postman/flush", "devnet_postmanFlush".into()),
        ("/postman/send_message_to_l2", "devnet_postmanSendMessageToL2".into()),
        ("/postman/consume_message_from_l2", "devnet_postmanConsumeMessageFromL2".into()),
        ("/predeployed_accounts", "devnet_getPredeployedAccounts".into()),
        ("/account_balance", "devnet_getAccountBalance".into()),
        ("/config", "devnet_getConfig".into()),
    ]);
    static ref RPC_METHOD_TO_HTTP_URI: HashMap<String, String> =
        HTTP_URI_TO_RPC_METHOD.iter().map(|(k, v)| (v.to_string(), String::from(*k))).collect();
    pub static ref DEFAULT_RESTRICTED_JSON_RPC_METHODS: Vec<String> = vec![
        "devnet_mint".into(),
        "devnet_load".into(),
        "devnet_restart".into(),
        "devnet_createBlock".into(),
        "devnet_abortBlocks".into(),
        "devnet_impersonateAccount".into(),
        "devnet_autoImpersonate".into(),
        "devnet_getPredeployedAccounts".into()
    ];
}

pub(crate) fn is_json_rpc_method_restricted(
    json_rpc_method: &String,
    restricted_methods: &[String],
) -> bool {
    if restricted_methods.contains(json_rpc_method) {
        return true;
    }

    match RPC_METHOD_TO_HTTP_URI.get(json_rpc_method) {
        Some(http_uri) => restricted_methods.contains(http_uri),
        None => false,
    }
}

pub(crate) fn is_uri_path_restricted(uri_path: &str, restricted_uris: &[String]) -> bool {
    if restricted_uris.contains(&uri_path.to_string()) {
        return true;
    }

    match HTTP_URI_TO_RPC_METHOD.get(uri_path) {
        Some(json_rpc_method) => restricted_uris.contains(json_rpc_method),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

    use super::DEFAULT_RESTRICTED_JSON_RPC_METHODS;
    use crate::restrictive_mode::{
        is_json_rpc_method_restricted, is_uri_path_restricted, RPC_METHOD_TO_HTTP_URI,
    };
    lazy_static! {
        static ref DEFAULT_RESTRICTED_HTTP_URIS: Vec<String> = DEFAULT_RESTRICTED_JSON_RPC_METHODS
            .iter()
            .filter_map(|method| RPC_METHOD_TO_HTTP_URI.get(method.as_str()))
            .map(|uri| uri.to_string())
            .collect();
    }
    #[test]
    fn test_provided_method_is_restricted() {
        assert_is_restricted("devnet_mint", &DEFAULT_RESTRICTED_HTTP_URIS);
        assert_is_restricted("/mint", &DEFAULT_RESTRICTED_HTTP_URIS);
        assert_is_restricted("devnet_mint", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted("/mint", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted("devnet_impersonateAccount", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted(
            "devnet_mint",
            &(["/mint", "dump", "devnet_mint"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()),
        );
    }

    #[test]
    fn test_provided_method_is_not_restricted() {
        assert_is_not_restricted("devnet_impersonateAccount", &DEFAULT_RESTRICTED_HTTP_URIS);
        assert_is_not_restricted("devnet_config", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);

        assert_is_not_restricted("devnet_config", &DEFAULT_RESTRICTED_HTTP_URIS);
        assert_is_not_restricted("/config", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_not_restricted("/config", &DEFAULT_RESTRICTED_HTTP_URIS);
    }

    fn assert_is_restricted(method: &str, restricted_methods: &[String]) {
        if method.contains('/') {
            assert!(is_uri_path_restricted(method, restricted_methods));
        } else {
            assert!(is_json_rpc_method_restricted(&method.to_string(), restricted_methods));
        }
    }
    fn assert_is_not_restricted(method: &str, restricted_methods: &[String]) {
        if method.contains('/') {
            assert!(!is_uri_path_restricted(method, restricted_methods));
        } else {
            assert!(!is_json_rpc_method_restricted(&method.to_string(), restricted_methods));
        }
    }
}
