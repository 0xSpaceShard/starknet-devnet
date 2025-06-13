use blockifier::transaction::transactions::ExecutableTransaction;
use ethers::types::H256;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;
use starknet_types::rpc::transactions::{Transaction, TransactionWithHash};
use tracing::trace;

use super::Starknet;
use crate::error::DevnetResult;

pub fn add_l1_handler_transaction(
    starknet: &mut Starknet,
    transaction: L1HandlerTransaction,
) -> DevnetResult<TransactionHash> {
    let executable_tx = transaction.create_sn_api_transaction(starknet.chain_id().to_felt())?;

    let transaction_hash = executable_tx.tx_hash.0;
    trace!("Executing L1 handler transaction [{:#064x}]", transaction_hash);

    let execution_info =
        blockifier::transaction::transaction_execution::Transaction::L1Handler(executable_tx)
            .execute(&mut starknet.pending_state.state, &starknet.block_context)?;

    starknet.handle_accepted_transaction(
        TransactionWithHash::new(transaction_hash, Transaction::L1Handler(transaction.clone())),
        execution_info,
    )?;

    // If L1 tx hash present, store the generated L2 tx hash in its messaging entry.
    // Not done as part of `handle_transaction_result` as it is specific to this tx type.
    if let Some(l1_tx_hash) = transaction.l1_transaction_hash {
        starknet
            .messaging
            .l1_to_l2_tx_hashes
            .entry(H256(*l1_tx_hash.as_bytes()))
            .or_default()
            .push(transaction_hash);
    }

    Ok(transaction_hash)
}

#[cfg(test)]
mod tests {
    // Constants taken from test_estimate_message_fee.rs.
    const WHITELISTED_L1_ADDRESS: &str = "0x8359E4B0152ed5A731162D3c7B0D8D56edB165A0";

    use nonzero_ext::nonzero;
    use starknet_rs_core::types::{Felt, TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::chain_id::ChainId;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::rpc::state::Balance;
    use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{
        self, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        ENTRYPOINT_NOT_FOUND_ERROR_ENCODED, ETH_ERC20_CONTRACT_ADDRESS,
        STRK_ERC20_CONTRACT_ADDRESS,
    };
    use crate::starknet::{Starknet, predeployed};
    use crate::state::CustomState;
    use crate::traits::{Deployed, HashIdentifiedMut};
    use crate::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use crate::utils::test_utils::{cairo_0_account_without_validations, dummy_key_pair};

    #[test]
    fn l1_handler_transaction_hash_computation() {
        let from_address = "0x000000000000000000000000be3C44c09bc1a3566F3e1CA12e5AbA0fA4Ca72Be";
        let to_address = "0x039dc79e64f4bb3289240f88e0bae7d21735bef0d1a51b2bf3c4730cb16983e1";
        let selector = "0x02f15cff7b0eed8b9beb162696cf4e3e0e35fa7032af69cd1b7d2ac67a13f40f";
        let nonce = 783082_u128;
        let fee = 30000_u128;
        let calldata: Vec<Felt> =
            vec![felt_from_prefixed_hex(from_address).unwrap(), Felt::ONE, Felt::TWO];

        let transaction = L1HandlerTransaction {
            contract_address: ContractAddress::new(felt_from_prefixed_hex(to_address).unwrap())
                .unwrap(),
            entry_point_selector: felt_from_prefixed_hex(selector).unwrap(),
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            ..Default::default()
        };

        let l1_handler_transaction_hash = transaction.compute_hash(ChainId::Testnet.to_felt());

        let transaction_hash = felt_from_prefixed_hex(
            "0x1b24ea8dd9e0cb603043958b27a8569635ea13568883cc155130591b7ffe37a",
        )
        .unwrap();

        assert_eq!(transaction.version, Felt::ZERO);
        assert_eq!(l1_handler_transaction_hash, transaction_hash);
    }

    #[test]
    fn l1_handler_transaction_successful_execution() {
        let (mut starknet, _account_address, contract_address, deposit_selector, _) = setup();

        let transaction = get_l1_handler_tx(
            felt_from_prefixed_hex(WHITELISTED_L1_ADDRESS).unwrap(),
            contract_address,
            deposit_selector,
            vec![Felt::from(11), Felt::from(9999)],
        );

        let transaction_hash = starknet.add_l1_handler_transaction(transaction).unwrap();

        let state_transaction = starknet.transactions.get_by_hash_mut(&transaction_hash).unwrap();

        assert_eq!(state_transaction.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(
            state_transaction.execution_result.status(),
            TransactionExecutionStatus::Succeeded
        );
    }

    #[test]
    fn l1_handler_transaction_not_l1_handler_entrypoint() {
        let (mut starknet, _account_address, contract_address, _, withdraw_selector) = setup();

        let transaction = get_l1_handler_tx(
            felt_from_prefixed_hex(WHITELISTED_L1_ADDRESS).unwrap(),
            contract_address,
            withdraw_selector,
            vec![Felt::from(11), Felt::from(9999)],
        );

        match starknet.add_l1_handler_transaction(transaction) {
            Err(crate::error::Error::ContractExecutionError(execution_error)) => {
                match execution_error {
                    crate::error::ContractExecutionError::Nested(
                        inner_contract_execution_error,
                    ) => {
                        assert_eq!(inner_contract_execution_error.selector, withdraw_selector);
                        assert_eq!(
                            inner_contract_execution_error.contract_address,
                            starknet_api::core::ContractAddress::try_from(contract_address)
                                .unwrap()
                        );

                        // check if there is a felt that corresponds to ENTRYPOINT_NOT_FOUND
                        assert!(
                            serde_json::to_string(&inner_contract_execution_error.error)
                                .unwrap()
                                .contains(&ENTRYPOINT_NOT_FOUND_ERROR_ENCODED.to_hex_string())
                        );
                    }
                    other_error_trace => {
                        panic!("Invalid error stack trace {:?}", other_error_trace)
                    }
                }
            }
            other => panic!("Wrong result: {other:?}"),
        }
    }

    /// Builds a `L1HandlerTransaction` from the given parameters. The nonce, fee and chain_id are
    /// fixed: nonce: 783082
    /// fee: 30000
    /// chain_id: ChainId::Testnet
    fn get_l1_handler_tx(
        from_address: Felt,
        contract_address: ContractAddress,
        entry_point_selector: Felt,
        payload: Vec<Felt>,
    ) -> L1HandlerTransaction {
        let nonce = 783082_u128;
        let fee = 30000_u128;

        let mut calldata = payload;
        calldata.insert(0, from_address);

        L1HandlerTransaction {
            contract_address,
            entry_point_selector,
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            ..Default::default()
        }
    }

    /// Initializes a starknet object with: l1l2 dummy contract that has two functions for
    /// messaging: `deposit` - the `#l1_handler` to receive message from L1, which accept 3 args
    /// `from_address`, `user`, `amount`. `withdraw` - the function to send message to L1 with 3
    /// args `MESSAGE_WITHDRAW=0`, user, `amount`.
    fn setup() -> (Starknet, ContractAddress, ContractAddress, Felt, Felt) {
        let mut starknet = Starknet::default();

        // deploy erc20 contract
        let eth_erc_20_contract =
            predeployed::tests::create_erc20_at_address(ETH_ERC20_CONTRACT_ADDRESS).unwrap();
        eth_erc_20_contract.deploy(&mut starknet.pending_state).unwrap();

        let strk_erc_20_contract =
            predeployed::tests::create_erc20_at_address(STRK_ERC20_CONTRACT_ADDRESS).unwrap();
        strk_erc_20_contract.deploy(&mut starknet.pending_state).unwrap();

        // deploy account contract
        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Balance::from(10000_u32),
            dummy_key_pair(),
            account_without_validations_class_hash,
            "Custom",
            ContractClass::Cairo0(account_without_validations_contract_class),
            eth_erc_20_contract.get_address(),
            strk_erc_20_contract.get_address(),
        )
        .unwrap();

        account.deploy(&mut starknet.pending_state).unwrap();

        // dummy contract
        let dummy_contract = dummy_cairo_l1l2_contract();
        let sn_api_class: starknet_api::deprecated_contract_class::ContractClass =
            dummy_contract.clone().try_into().unwrap();

        let withdraw_selector = get_selector_from_name("withdraw").unwrap();
        let deposit_selector = get_selector_from_name("deposit").unwrap();

        // check if withdraw function is present in the contract class
        sn_api_class
            .entry_points_by_type
            .get(&starknet_api::contract_class::EntryPointType::External)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == withdraw_selector)
            .unwrap();

        // check if deposit function is present in the contract class
        sn_api_class
            .entry_points_by_type
            .get(&starknet_api::contract_class::EntryPointType::L1Handler)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == deposit_selector)
            .unwrap();

        let dummy_contract_address = ContractAddress::new(Felt::from(5)).unwrap();
        let dummy_contract_class_hash = dummy_contract.generate_hash().unwrap();

        // declare dummy contract
        starknet
            .pending_state
            .declare_contract_class(dummy_contract_class_hash, None, dummy_contract.into())
            .unwrap();

        // deploy dummy contract
        starknet
            .pending_state
            .predeploy_contract(dummy_contract_address, dummy_contract_class_hash)
            .unwrap();
        starknet.block_context = Starknet::init_block_context(
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            nonzero!(1u128),
            constants::ETH_ERC20_CONTRACT_ADDRESS,
            constants::STRK_ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
            DEVNET_DEFAULT_STARTING_BLOCK_NUMBER,
        );

        starknet.restart_pending_block().unwrap();

        (
            starknet,
            account.get_address(),
            dummy_contract_address,
            deposit_selector,
            withdraw_selector,
        )
    }
}
