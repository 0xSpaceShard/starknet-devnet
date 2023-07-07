use super::RpcResult;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::models::transaction::BroadcastedDeclareTransaction;

impl JsonRpcHandler {
    pub(crate) async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> RpcResult<()> {
        match request {
            BroadcastedDeclareTransaction::V1(_) => todo!(),
            BroadcastedDeclareTransaction::V2(_) => todo!(),
        }

        Ok(())
    }
}
