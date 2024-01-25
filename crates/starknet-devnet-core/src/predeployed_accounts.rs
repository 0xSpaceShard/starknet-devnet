use starknet_rs_ff::FieldElement;
use starknet_rs_signers::SigningKey;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, Key};

use crate::account::Account;
use crate::error::DevnetResult;
use crate::traits::AccountGenerator;
use crate::utils::random_number_generator::generate_u128_random_numbers;

#[derive(Default)]
pub(crate) struct PredeployedAccounts {
    seed: u32,
    initial_balance: Felt,
    eth_fee_token_address: ContractAddress,
    strk_fee_token_address: ContractAddress,
    accounts: Vec<Account>,
}

impl PredeployedAccounts {
    pub(crate) fn new(
        seed: u32,
        initial_balance: Felt,
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
        let private_key_field_element = FieldElement::from(*private_key);

        Key::from(
            SigningKey::from_secret_scalar(private_key_field_element).verifying_key().scalar(),
        )
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
        contract_class: ContractClass,
    ) -> DevnetResult<&Vec<Self::Acc>> {
        let private_keys = self.generate_private_keys(number_of_accounts);

        for private_key in private_keys {
            let account = Account::new(
                self.initial_balance,
                self.generate_public_key(&private_key),
                private_key,
                class_hash,
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
    use rand::{thread_rng, Rng};
    use starknet_types::felt::Felt;

    use crate::predeployed_accounts::PredeployedAccounts;
    use crate::utils::test_utils::dummy_contract_address;

    #[test]
    fn private_key_from_different_seeds_should_be_different_and_equal_from_equal_seeds() {
        let mut rng = thread_rng();

        // looping 10000 times to make sure that at least once the generated seeds are equal
        for _ in 0..10000 {
            let seed1 = rng.gen::<u32>();
            let private_key1 = PredeployedAccounts::new(
                seed1,
                Felt::from(1),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            let seed2 = rng.gen::<u32>();
            let private_key2 = PredeployedAccounts::new(
                seed2,
                Felt::from(1),
                dummy_contract_address(),
                dummy_contract_address(),
            )
            .generate_private_keys(1)[0];

            if seed1 == seed2 {
                assert_eq!(private_key1, private_key2);
            } else {
                assert_ne!(private_key1, private_key2);
            }
        }
    }
}
