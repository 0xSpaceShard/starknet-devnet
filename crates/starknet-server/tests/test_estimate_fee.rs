pub mod common;

mod estimate_fee_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_accounts::{AccountFactory, OpenZeppelinAccountFactory};
    use starknet_rs_core::types::{FeeEstimate, FieldElement};

    use crate::common::constants::CHAIN_ID;
    use crate::common::util::BackgroundDevnet;

    fn assert_fee_estimation(fee_estimation: &FeeEstimate) {
        assert_eq!(
            fee_estimation.gas_price * fee_estimation.gas_consumed,
            fee_estimation.overall_fee
        );
        assert!(fee_estimation.overall_fee > 0u64, "Checking fee_estimation: {fee_estimation:?}");
    }

    #[tokio::test]
    async fn estimate_fee_of_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_private_key = "0xc248668388dbe9acdfa3bc734cc2d57a";
        let new_account_signer = starknet_rs_signers::LocalWallet::from(
            starknet_rs_signers::SigningKey::from_secret_scalar(
                FieldElement::from_hex_be(new_account_private_key).unwrap(),
            ),
        );
        let new_account_nonce = FieldElement::ZERO;

        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            new_account_signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        // fund address
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let deployment = account_factory.deploy(salt);
        let deployment_address = deployment.address();
        let fee_estimation = account_factory
            .deploy(salt)
            .fee_estimate_multiplier(1.0)
            .nonce(new_account_nonce)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // fund the account before deployment
        let mint_amount = fee_estimation.overall_fee as u128 * 2;
        devnet.mint(deployment_address, mint_amount).await;

        // TODO uncomment the following section once starknet_in_rust fixes max_fee checking
        // try sending with insufficient max fee
        // let insufficient_max_fee = fee_estimation.overall_fee * 9 / 10; // 90% of estimate - not
        // enough let unsuccessful_deployment_tx = account_factory
        // .deploy(salt)
        // .max_fee(FieldElement::from(insufficient_max_fee))
        // .nonce(new_account_nonce)
        // .send()
        // .await
        // .unwrap();
        // todo!("Assert the tx is not accepted");

        // try sending with sufficient max fee
        let sufficient_max_fee = fee_estimation.overall_fee * 11 / 10;
        let _result = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from(sufficient_max_fee))
            .nonce(new_account_nonce)
            .send()
            .await
            .expect("Should deploy with sufficient fee");
        // TODO assert tx is accepted
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v1() {
        todo!();
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v2() {
        todo!();
    }

    #[tokio::test]
    async fn estimate_fee_of_invoke() {
        todo!();
    }

    #[tokio::test]
    async fn estimate_fee_of_multiple_txs() {
        todo!();
    }
}
