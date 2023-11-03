// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_restart {
    use hyper::StatusCode;
    use starknet_rs_core::types::{FieldElement, StarknetError};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::devnet::BackgroundDevnet;

    #[tokio::test]
    async fn assert_restartable() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.restart().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn assert_tx_not_present_after_restart() {
        // generate tx
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let dummy_address = FieldElement::from_hex_be("0x1").unwrap();
        let mint_hash = devnet.mint(dummy_address, 100).await;
        assert!(devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.is_ok());

        let restart_resp = devnet.restart().await.unwrap();
        assert_eq!(restart_resp.status(), StatusCode::OK);

        match devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.unwrap_err() {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            other => panic!("Invalid error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn assert_storage_restarted() {
        todo!();
    }

    #[tokio::test]
    async fn assert_gas_price_unaffected_by_restart() {
        todo!();
    }

    #[tokio::test]
    async fn assert_predeployed_account_still_prefunded() {
        todo!();
    }
}
