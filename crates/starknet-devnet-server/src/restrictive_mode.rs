use std::collections::HashMap;

// Constants for URIs
const URI_DUMP: &str = "/dump";
const URI_LOAD: &str = "/load";
const URI_SET_TIME: &str = "/set_time";
const URI_INCREASE_TIME: &str = "/increase_time";
const URI_CREATE_BLOCK: &str = "/create_block";
const URI_ABORT_BLOCKS: &str = "/abort_blocks";
const URI_RESTART: &str = "/restart";
const URI_MINT: &str = "/mint";
const URI_POSTMAN_LOAD_L1_MESSAGING_CONTRACT: &str = "/postman/load_l1_messaging_contract";
const URI_POSTMAN_FLUSH: &str = "/postman/flush";
const URI_POSTMAN_SEND_MESSAGE_TO_L2: &str = "/postman/send_message_to_l2";
const URI_POSTMAN_CONSUME_MESSAGE_FROM_L2: &str = "/postman/consume_message_from_l2";
const URI_PREDEPLOYED_ACCOUNTS: &str = "/predeployed_accounts";
const URI_ACCOUNT_BALANCE: &str = "/account_balance";
const URI_CONFIG: &str = "/config";

// Constants for JSON RPC method names
const RPC_METHOD_DUMP: &str = "devnet_dump";
const RPC_METHOD_LOAD: &str = "devnet_load";
const RPC_METHOD_SET_TIME: &str = "devnet_setTime";
const RPC_METHOD_INCREASE_TIME: &str = "devnet_increaseTime";
const RPC_METHOD_CREATE_BLOCK: &str = "devnet_createBlock";
const RPC_METHOD_ABORT_BLOCKS: &str = "devnet_abortBlocks";
const RPC_METHOD_RESTART: &str = "devnet_restart";
const RPC_METHOD_MINT: &str = "devnet_mint";
const RPC_METHOD_POSTMAN_LOAD: &str = "devnet_postmanLoad";
const RPC_METHOD_POSTMAN_FLUSH: &str = "devnet_postmanFlush";
const RPC_METHOD_POSTMAN_SEND_MESSAGE_TO_L2: &str = "devnet_postmanSendMessageToL2";
const RPC_METHOD_POSTMAN_CONSUME_MESSAGE_FROM_L2: &str = "devnet_postmanConsumeMessageFromL2";
const RPC_METHOD_GET_PREDEPLOYED_ACCOUNTS: &str = "devnet_getPredeployedAccounts";
const RPC_METHOD_GET_ACCOUNT_BALANCE: &str = "devnet_getAccountBalance";
const RPC_METHOD_GET_CONFIG: &str = "devnet_getConfig";
const RPC_METHOD_IMPERSONATE_ACCOUNT: &str = "devnet_impersonateAccount";
const RPC_METHOD_AUTO_IMPERSONATE: &str = "devnet_autoImpersonate";

use lazy_static::lazy_static;
lazy_static! {
    static ref HTTP_URI_TO_RPC_METHOD: HashMap<&'static str, String> = HashMap::from([
        (URI_DUMP, RPC_METHOD_DUMP.into()),
        (URI_LOAD, RPC_METHOD_LOAD.into()),
        (URI_SET_TIME, RPC_METHOD_SET_TIME.into()),
        (URI_INCREASE_TIME, RPC_METHOD_INCREASE_TIME.into()),
        (URI_CREATE_BLOCK, RPC_METHOD_CREATE_BLOCK.into()),
        (URI_ABORT_BLOCKS, RPC_METHOD_ABORT_BLOCKS.into()),
        (URI_RESTART, RPC_METHOD_RESTART.into()),
        (URI_MINT, RPC_METHOD_MINT.into()),
        (URI_POSTMAN_LOAD_L1_MESSAGING_CONTRACT, RPC_METHOD_POSTMAN_LOAD.into()),
        (URI_POSTMAN_FLUSH, RPC_METHOD_POSTMAN_FLUSH.into()),
        (URI_POSTMAN_SEND_MESSAGE_TO_L2, RPC_METHOD_POSTMAN_SEND_MESSAGE_TO_L2.into()),
        (URI_POSTMAN_CONSUME_MESSAGE_FROM_L2, RPC_METHOD_POSTMAN_CONSUME_MESSAGE_FROM_L2.into()),
        (URI_PREDEPLOYED_ACCOUNTS, RPC_METHOD_GET_PREDEPLOYED_ACCOUNTS.into()),
        (URI_ACCOUNT_BALANCE, RPC_METHOD_GET_ACCOUNT_BALANCE.into()),
        (URI_CONFIG, RPC_METHOD_GET_CONFIG.into()),
    ]);
    static ref RPC_METHOD_TO_HTTP_URI: HashMap<String, String> =
        HTTP_URI_TO_RPC_METHOD.iter().map(|(k, v)| (v.to_string(), String::from(*k))).collect();
    pub static ref DEFAULT_RESTRICTED_JSON_RPC_METHODS: Vec<String> = vec![
        RPC_METHOD_MINT.into(),
        RPC_METHOD_LOAD.into(),
        RPC_METHOD_RESTART.into(),
        RPC_METHOD_CREATE_BLOCK.into(),
        RPC_METHOD_ABORT_BLOCKS.into(),
        RPC_METHOD_IMPERSONATE_ACCOUNT.into(),
        RPC_METHOD_AUTO_IMPERSONATE.into(),
        RPC_METHOD_GET_PREDEPLOYED_ACCOUNTS.into()
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
