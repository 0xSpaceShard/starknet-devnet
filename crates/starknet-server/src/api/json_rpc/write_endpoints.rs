use crate::api::json_rpc::JsonRpcHandler;

use super::RpcResult;

impl JsonRpcHandler {
    pub(crate) async fn add_declare_transaction(&self) -> RpcResult<()> {
        Ok(())
    }
}