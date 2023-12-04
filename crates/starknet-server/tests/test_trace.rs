pub mod common;

mod trace_tests {
    use starknet_core::constants::{CHARGEABLE_ACCOUNT_ADDRESS, ERC20_CONTRACT_ADDRESS};
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;

    static DUMMY_ADDRESS: u128 = 123;
    static DUMMY_AMOUNT: u128 = 2;

    #[tokio::test]
    async fn get_invoke_trace() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        let mint_tx_trace = devnet.json_rpc_client.trace_transaction(mint_tx_hash).await.unwrap();

        if let starknet_rs_core::types::TransactionTrace::Invoke(invoke_trace) = mint_tx_trace {
            let validate_invocation = invoke_trace.validate_invocation.unwrap();
            assert_eq!(
                validate_invocation.contract_address,
                FieldElement::from_hex_be(CHARGEABLE_ACCOUNT_ADDRESS).unwrap()
            );
            assert_eq!(validate_invocation.calldata[6], FieldElement::from(DUMMY_ADDRESS));
            assert_eq!(validate_invocation.calldata[7], FieldElement::from(DUMMY_AMOUNT));

            if let starknet_rs_core::types::ExecuteInvocation::Success(execute_invocation) =
                invoke_trace.execute_invocation
            {
                assert_eq!(
                    execute_invocation.contract_address,
                    FieldElement::from_hex_be(CHARGEABLE_ACCOUNT_ADDRESS).unwrap()
                );
                assert_eq!(execute_invocation.calldata[6], FieldElement::from(DUMMY_ADDRESS));
                assert_eq!(execute_invocation.calldata[7], FieldElement::from(DUMMY_AMOUNT));
            }

            assert_eq!(
                invoke_trace.fee_transfer_invocation.unwrap().contract_address,
                FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction trace from {mint_tx_trace:?}");
        }
    }

    // TODO: Add DeclareTransactionTrace test
    // TODO: Add DeployAccountTransactionTrace test and check constructor_invocation
}
