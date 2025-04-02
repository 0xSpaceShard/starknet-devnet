use starknet_rs_signers::SigningKey;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Key};
use starknet_types::rpc::state::Balance;

use crate::account::{Account, KeyPair};
use crate::error::DevnetResult;
use crate::traits::AccountGenerator;
use crate::utils::random_number_generator::generate_u128_random_numbers;

#[derive(Default)]
pub(crate) struct PredeployedAccounts {
    seed: u32,
    initial_balance: Balance,
    eth_fee_token_address: ContractAddress,
    strk_fee_token_address: ContractAddress,
    accounts: Vec<Account>,
}

impl PredeployedAccounts {
    pub(crate) fn new(
        seed: u32,
        initial_balance: Balance,
        eth_fee_token_address: ContractAddress,
        strk_fee_token_address: ContractAddress,
    ) -> Self {
        Self {
            seed,
            initial_balance,
            eth_fee_token_address,
            strk_fee_token_address,
            accounts: Vec::new(),
        }
    }
}

impl PredeployedAccounts {
    fn generate_private_keys(&self, number_of_accounts: u8) -> Vec<Key> {
        let random_numbers = generate_u128_random_numbers(self.seed, number_of_accounts);
        random_numbers.into_iter().map(Key::from).collect::<Vec<Key>>()
    }

    fn generate_public_key(&self, private_key: &Key) -> Key {
        Key::from(SigningKey::from_secret_scalar(*private_key).verifying_key().scalar())
    }

    pub fn get_accounts(&self) -> &Vec<Account> {
        &self.accounts
    }
}

impl AccountGenerator for PredeployedAccounts {
    type Acc = Account;

    fn generate_accounts(
        &mut self,
        number_of_accounts: u8,
        class_hash: ClassHash,
        contract_class: &ContractClass,
    ) -> DevnetResult<&Vec<Self::Acc>> {
        let private_keys = self.generate_private_keys(number_of_accounts);

        for private_key in private_keys {
            let account = Account::new(
                self.initial_balance.clone(),
                KeyPair { public_key: self.generate_public_key(&private_key), private_key },
                class_hash,
                "Custom",
                contract_class.clone(),
                self.eth_fee_token_address,
                self.strk_fee_token_address,
            )?;
            self.accounts.push(account);
        }

        Ok(&self.accounts)
    }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, thread_rng};
    use starknet_types::rpc::state::Balance;

    use crate::predeployed_accounts::PredeployedAccounts;
    use crate::utils::test_utils::dummy_contract_address;

    #[test]
    fn private_key_from_equal_seeds_have_to_be_equal() {
        for _ in 0..1000 {
            let seed = thread_rng().gen::<u32>();

            let private_key1 = PredeployedAccounts::new(
                seed,
                Balance::from(1_u8),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            let private_key2 = PredeployedAccounts::new(
                seed,
                Balance::from(1_u8),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            assert_eq!(private_key1, private_key2);
        }
    }

    #[test]
    fn private_key_from_different_seeds_have_to_be_different() {
        let mut rng = thread_rng();

        for _ in 0..1000 {
            let mut seed1;
            let mut seed2;

            // get two different seeds
            loop {
                seed1 = rng.gen::<u32>();
                seed2 = rng.gen::<u32>();
                if seed1 != seed2 {
                    break;
                }
            }

            let private_key1 = PredeployedAccounts::new(
                seed1,
                Balance::from(1_u8),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            let private_key2 = PredeployedAccounts::new(
                seed2,
                Balance::from(1_u8),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            assert_ne!(private_key1, private_key2);
        }
    }
}
