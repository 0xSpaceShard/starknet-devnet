pub static DEFAULT_RESTRICTED_JSON_RPC_METHODS: &[&str] = &[
    "devnet_mint",
    "devnet_load",
    "devnet_restart",
    "devnet_createBlock",
    "devnet_abortBlocks",
    "devnet_impersonateAccount",
    "devnet_autoImpersonate",
    "devnet_getPredeployedAccounts",
];

pub(crate) fn is_json_rpc_method_restricted<T: AsRef<str>, U: AsRef<str>>(
    json_rpc_method: T,
    restricted_methods: &[U],
) -> bool {
    restricted_methods.iter().any(|method| method.as_ref() == json_rpc_method.as_ref())
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_RESTRICTED_JSON_RPC_METHODS;
    use crate::api::models::JsonRpcRequest;
    use crate::restrictive_mode::is_json_rpc_method_restricted;
    #[test]
    fn test_provided_method_is_restricted() {
        assert_is_restricted("devnet_mint", DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted("devnet_impersonateAccount", DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted(
            "devnet_mint",
            &["devnet_abortBlocks", "devnet_CreateBlocks", "devnet_mint"],
        );
    }

    #[test]
    fn test_provided_method_is_not_restricted() {
        assert_is_not_restricted("devnet_getConfig", DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_not_restricted("devnet_getAccountBalance", DEFAULT_RESTRICTED_JSON_RPC_METHODS);
    }

    #[test]
    fn test_default_restricted_exist() {
        let json_rpc_methods = JsonRpcRequest::all_variants_serde_renames();
        for method in DEFAULT_RESTRICTED_JSON_RPC_METHODS {
            assert!(json_rpc_methods.contains(&method.to_string()));
        }
    }

    fn assert_is_restricted(method: &str, restricted_methods: &[&str]) {
        assert!(is_json_rpc_method_restricted(method, restricted_methods));
    }

    fn assert_is_not_restricted(method: &str, restricted_methods: &[&str]) {
        assert!(!is_json_rpc_method_restricted(method, restricted_methods));
    }
}
