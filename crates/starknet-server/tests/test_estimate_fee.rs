pub mod common;

mod estimate_fee_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_accounts::{AccountFactory, OpenZeppelinAccountFactory};
    use starknet_rs_core::types::FieldElement;

    use crate::common::{constants::CHAIN_ID, util::BackgroundDevnet};

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
            .nonce(new_account_nonce.clone())
            .estimate_fee()
            .await
            .unwrap();

        // fund the account before deployment
        let mint_amount = 10u128;
        devnet.mint(deployment_address, mint_amount).await;

        // try sending with insufficient max fee
        // println!("DEBUG fee estimation: {fee_estimation:?}");
        // let insufficient_max_fee = 1u32; // TODO TMP 50% - not enough
        // let error = account_factory
        //     .deploy(salt)
        //     .max_fee(FieldElement::from(insufficient_max_fee))
        //     .nonce(new_account_nonce.clone())
        //     .send()
        //     .await
        //     .expect_err("Should have failed");
        // assert!(error.to_string().contains("TODO DUMMY MESSAGE"), "Checking {error}");

        // try sending with sufficient max fee
        let sufficient_max_fee = fee_estimation.overall_fee; // TODO multiply with factor > 1 or leave at 100%
        let result = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from(2_000_000_000_000_000u128))
            .nonce(new_account_nonce.clone())
            .send()
            .await
            .expect("Should deploy with sufficient fee");

        todo!("DEBUG TODO result: {result:?}");
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
}
