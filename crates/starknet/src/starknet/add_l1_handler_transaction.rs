use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::{L1HandlerTransaction, Transaction};
use tracing::trace;

use super::Starknet;
use crate::error::DevnetResult;

pub fn add_l1_handler_transaction(
    starknet: &mut Starknet,
    transaction: L1HandlerTransaction,
) -> DevnetResult<TransactionHash> {
    let transaction_hash = transaction.transaction_hash;
    trace!("Executing L1 handler transaction [{:#064x}]", transaction.transaction_hash);

    let blockifier_transaction = transaction.create_blockifier_transaction()?;

    // Fees are charges on L1 as `L1HandlerTransaction` is not executed by an
    // account, but directly by the sequencer.
    // https://docs.starknet.io/documentation/architecture_and_concepts/Network_Architecture/messaging-mechanism/#l1-l2-message-fees
    let charge_fee = false;
    let validate = true;

    let blockifier_execution_result = blockifier_transaction.execute(
        &mut starknet.state.state,
        &starknet.block_context,
        charge_fee,
        validate,
    );

    starknet.handle_transaction_result(
        Transaction::L1Handler(transaction),
        blockifier_execution_result,
    )?;

    Ok(transaction_hash)
}

#[cfg(test)]
mod tests {
    // Constants taken from test_estimate_message_fee.rs.
    const WHITELISTED_L1_ADDRESS: &str = "0x8359E4B0152ed5A731162D3c7B0D8D56edB165A0";

    use blockifier::execution::errors::{EntryPointExecutionError, PreExecutionError};
    use blockifier::transaction::errors::TransactionExecutionError::ExecutionError;
    use starknet_api::hash::StarkFelt;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::chain_id::ChainId;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transactions::L1HandlerTransaction;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut, StateChanger};
    use crate::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use crate::utils::test_utils::{
        cairo_0_account_without_validations, dummy_felt, get_bytes_from_u32,
    };

    #[test]
    fn l1_handler_transaction_hash_computation() {
        let from_address = "0x000000000000000000000000be3C44c09bc1a3566F3e1CA12e5AbA0fA4Ca72Be";
        let to_address = "0x039dc79e64f4bb3289240f88e0bae7d21735bef0d1a51b2bf3c4730cb16983e1";
        let selector = "0x02f15cff7b0eed8b9beb162696cf4e3e0e35fa7032af69cd1b7d2ac67a13f40f";
        let nonce = 783082_u128;
        let fee = 30000_u128;
        let calldata: Vec<Felt> =
            vec![Felt::from_prefixed_hex_str(from_address).unwrap(), 1.into(), 2.into()];

        let transaction = L1HandlerTransaction {
            contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(to_address).unwrap(),
            )
            .unwrap(),
            entry_point_selector: Felt::from_prefixed_hex_str(selector).unwrap(),
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            ..Default::default()
        }
        .with_hash(ChainId::Testnet.to_felt());

        let transaction_hash = Felt::from_prefixed_hex_str(
            "0x6182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b",
        )
        .unwrap();

        assert_eq!(transaction.version, Felt::from(0));
        assert_eq!(transaction.transaction_hash, transaction_hash);
    }

    #[test]
    fn l1_handler_transaction_successful_execution() {
        let (mut starknet, _account_address, contract_address, deposit_selector, _) = setup();

        let transaction = get_l1_handler_tx(
            Felt::from_prefixed_hex_str(WHITELISTED_L1_ADDRESS).unwrap(),
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
            Felt::from_prefixed_hex_str(WHITELISTED_L1_ADDRESS).unwrap(),
            contract_address,
            withdraw_selector,
            vec![Felt::from(11), Felt::from(9999)],
        );

        let result = starknet.add_l1_handler_transaction(transaction);

        match result {
            Err(crate::error::Error::BlockifierTransactionError(ExecutionError(
                EntryPointExecutionError::PreExecutionError(PreExecutionError::EntryPointNotFound(
                    selector,
                )),
            ))) => {
                assert_eq!(selector.0, withdraw_selector.into())
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
        .with_hash(ChainId::Testnet.to_felt())
    }

    /// Initializes a starknet object with: l1l2 dummy contract that has two functions for
    /// messaging: `deposit` - the `#l1_handler` to receive message from L1, which accept 3 args
    /// `from_address`, `user`, `amount`. `withdraw` - the function to send message to L1 with 3
    /// args `MESSAGE_WITHDRAW=0`, user, `amount`.
    fn setup() -> (Starknet, ContractAddress, ContractAddress, Felt, Felt) {
        let mut starknet = Starknet::default();

        // deploy erc20 contract
        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        // deploy account contract
        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Felt::from(10000),
            dummy_felt(),
            dummy_felt(),
            account_without_validations_class_hash,
            ContractClass::Cairo0(account_without_validations_contract_class),
            erc_20_contract.get_address(),
        )
        .unwrap();

        account.deploy(&mut starknet.state).unwrap();
        account.set_initial_balance(&mut starknet.state).unwrap();

        // dummy contract
        let dummy_contract: Cairo0ContractClass = dummy_cairo_l1l2_contract().into();
        let blockifier = blockifier::execution::contract_class::ContractClassV0::try_from(
            dummy_contract.clone(),
        )
        .unwrap();
        let withdraw_selector: StarkFelt =
            Felt::from(get_selector_from_name("withdraw").unwrap()).into();
        let deposit_selector: StarkFelt =
            Felt::from(get_selector_from_name("deposit").unwrap()).into();

        // check if withdraw function is present in the contract class
        blockifier
            .entry_points_by_type
            .get(&starknet_api::deprecated_contract_class::EntryPointType::External)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == withdraw_selector)
            .unwrap();

        // check if deposit function is present in the contract class
        blockifier
            .entry_points_by_type
            .get(&starknet_api::deprecated_contract_class::EntryPointType::L1Handler)
            .unwrap()
            .iter()
            .find(|el| el.selector.0 == deposit_selector)
            .unwrap();

        let mut address_bytes = get_bytes_from_u32(5);
        address_bytes.reverse();

        let dummy_contract_address =
            ContractAddress::new(Felt::new(address_bytes).unwrap()).unwrap();
        let dummy_contract_class_hash = dummy_contract.generate_hash().unwrap();

        // declare dummy contract
        starknet
            .state
            .declare_contract_class(dummy_contract_class_hash, dummy_contract.into())
            .unwrap();

        // deploy dummy contract
        starknet.state.deploy_contract(dummy_contract_address, dummy_contract_class_hash).unwrap();
        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (
            starknet,
            account.get_address(),
            dummy_contract_address,
            deposit_selector.into(),
            withdraw_selector.into(),
        )
    }
}
