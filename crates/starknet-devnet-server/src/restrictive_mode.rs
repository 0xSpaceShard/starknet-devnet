// Constants for JSON RPC method names
const RPC_METHOD_LOAD: &str = "devnet_load";
const RPC_METHOD_CREATE_BLOCK: &str = "devnet_createBlock";
const RPC_METHOD_ABORT_BLOCKS: &str = "devnet_abortBlocks";
const RPC_METHOD_RESTART: &str = "devnet_restart";
const RPC_METHOD_MINT: &str = "devnet_mint";
const RPC_METHOD_GET_PREDEPLOYED_ACCOUNTS: &str = "devnet_getPredeployedAccounts";
const RPC_METHOD_IMPERSONATE_ACCOUNT: &str = "devnet_impersonateAccount";
const RPC_METHOD_AUTO_IMPERSONATE: &str = "devnet_autoImpersonate";

use lazy_static::lazy_static;
lazy_static! {
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
    restricted_methods.contains(json_rpc_method)
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_RESTRICTED_JSON_RPC_METHODS;
    use crate::api::json_rpc::JsonRpcRequest;
    use crate::restrictive_mode::is_json_rpc_method_restricted;
    #[test]
    fn test_provided_method_is_restricted() {
        assert_is_restricted("devnet_mint", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted("devnet_impersonateAccount", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_restricted(
            "devnet_mint",
            &(["devnet_abortBlocks", "devnet_CreateBlocks", "devnet_mint"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()),
        );
    }

    #[test]
    fn test_provided_method_is_not_restricted() {
        assert_is_not_restricted("devnet_getConfig", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
        assert_is_not_restricted("devnet_getAccountBalance", &DEFAULT_RESTRICTED_JSON_RPC_METHODS);
    }

    #[test]
    fn test_default_restricted_exist() {
        let json_rpc_methods = JsonRpcRequest::all_variants_serde_renames();
        for method in DEFAULT_RESTRICTED_JSON_RPC_METHODS.iter() {
            assert!(json_rpc_methods.contains(method));
        }
    }

    fn assert_is_restricted(method: &str, restricted_methods: &[String]) {
        assert!(is_json_rpc_method_restricted(&method.to_string(), restricted_methods));
    }
    fn assert_is_not_restricted(method: &str, restricted_methods: &[String]) {
        assert!(!is_json_rpc_method_restricted(&method.to_string(), restricted_methods));
    }
}
