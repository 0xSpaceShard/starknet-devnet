use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::state::state_api::StateReader;
use starknet_in_rust::SierraContractClass;
use starknet_rs_core::types::BlockId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
use starknet_types::felt::{ClassHash, Felt};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;

pub fn get_class_hash_at_impl(
    starknet: &Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> DevnetResult<ClassHash> {
    let address = contract_address.into();
    let state = starknet.get_state_at(&block_id)?;
    let class_hash = state.state.get_class_hash_at(&address)?.into();

    if class_hash == Felt::default() {
        return Err(Error::StateError(StateError::NoneContractState(address)));
    }

    Ok(class_hash)
}

fn get_sierra_class(
    starknet: &Starknet,
    class_hash: &ClassHash,
) -> DevnetResult<SierraContractClass> {
    match starknet.state.contract_classes.get(class_hash) {
        Some(contract) => Ok(contract.clone().try_into()?),
        None => Err(Error::ContractNotFound),
    }
}

fn get_cairo_0_class(
    starknet: &Starknet,
    class_hash: &ClassHash,
) -> DevnetResult<Cairo0ContractClass> {
    match starknet.state.contract_classes.get(class_hash) {
        Some(contract) => Ok(contract.clone().try_into()?),
        None => Err(Error::ContractNotFound),
    }
}

pub fn get_class_impl(
    starknet: &Starknet,
    block_id: BlockId,
    class_hash: ClassHash,
) -> DevnetResult<ContractClass> {
    let state = starknet.get_state_at(&block_id)?;

    match state.state.get_contract_class(&class_hash.into()) {
        Ok(compiled_class) => match compiled_class {
            CompiledClass::Casm(_) => Ok(get_sierra_class(starknet, &class_hash)?.into()),
            CompiledClass::Deprecated(_) => Ok(get_cairo_0_class(starknet, &class_hash)?.into()),
        },
        Err(err) => Err(err.into()),
    }
}

pub fn get_class_at_impl(
    starknet: &Starknet,
    block_id: BlockId,
    contract_address: ContractAddress,
) -> DevnetResult<ContractClass> {
    let class_hash = starknet.get_class_hash_at(block_id, contract_address)?;
    starknet.get_class(block_id, class_hash)
}

#[cfg(test)]
mod tests {

    use starknet_rs_core::types::BlockId;
    use starknet_types::contract_class::{Cairo0Json, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::state::state_diff::StateDiff;
    use crate::traits::{Accounted, Deployed};
    use crate::utils::test_utils::{dummy_broadcasted_declare_transaction_v2, dummy_felt};

    fn setup(acc_balance: Option<u128>) -> (Starknet, Account) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();

        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(acc_balance.unwrap_or(100)),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class.into(),
            erc_20_contract.get_address(),
        )
        .unwrap();

        acc.deploy(&mut starknet.state).unwrap();
        acc.set_initial_balance(&mut starknet.state).unwrap();

        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::get_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        )
        .unwrap();

        starknet.restart_pending_block().unwrap();

        (starknet, acc)
    }

    #[test]
    fn get_sierra_class() {
        let (mut starknet, account) = setup(Some(100000000));

        let declare_txn = dummy_broadcasted_declare_transaction_v2(&account.account_address);

        let expected: ContractClass = declare_txn.contract_class.clone().into();
        let (_, class_hash) = starknet.add_declare_transaction_v2(declare_txn).unwrap();

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let contract_class =
            starknet.get_class(BlockId::Number(block_number.0), class_hash).unwrap();

        assert_eq!(contract_class, expected)
    }

    #[test]
    fn get_class_hash_at_generated_accounts() {
        let (mut starknet, account) = setup(Some(100000000));

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let block_id = BlockId::Number(block_number.0);

        let class_hash = starknet.get_class_hash_at(block_id, account.account_address).unwrap();
        let expected = account.class_hash;
        assert_eq!(class_hash, expected);
    }

    #[test]
    fn get_class_at_generated_accounts() {
        let (mut starknet, account) = setup(Some(100000000));

        starknet.generate_new_block(StateDiff::default(), starknet.state.clone()).unwrap();
        starknet.generate_pending_block().unwrap();

        let block_number = starknet.get_latest_block().unwrap().block_number();
        let block_id = BlockId::Number(block_number.0);

        let contract_class = starknet.get_class_at(block_id, account.account_address).unwrap();
        assert_eq!(contract_class, account.contract_class);
    }
}
