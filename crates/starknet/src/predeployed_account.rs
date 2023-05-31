use starknet_api::hash::StarkFelt;
use starknet_rs_ff::FieldElement;
use starknet_rs_signers::SigningKey;

use crate::{
    account::Account,
    error::Error,
    traits::AccountGenerator,
    types::{Balance, DevnetResult, Felt, Key},
    utils::generate_u128_random_numbers,
};

struct PredeployedAccount {
    seed: u32,
    initial_balance: u32,
}

impl PredeployedAccount {
    fn new(seed: u32, initial_balance: u32) -> Self {
        Self { seed, initial_balance }
    }
}

impl PredeployedAccount {
    fn generate_private_keys(&self, number_of_accounts: u8) -> DevnetResult<Vec<Key>> {
        let random_numbers = generate_u128_random_numbers(self.seed, number_of_accounts)?;
        let private_keys = random_numbers.into_iter().map(|num| Key::from(num)).collect::<Vec<Key>>();

        Ok(private_keys)
    }

    fn generate_public_key(&self, private_key: &Key) -> DevnetResult<Key> {
        let private_key_field_element = FieldElement::from(private_key.clone());

        let public_key = Key::from(SigningKey::from_secret_scalar(private_key_field_element).verifying_key().scalar());

        Ok(public_key)
    }
}

impl AccountGenerator for PredeployedAccount {
    type Acc = Account;

    fn generate_accounts(&self, number_of_accounts: u8) -> DevnetResult<Vec<Self::Acc>>
    where
        Self::Acc: crate::traits::Accounted,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::serde_utils::bytes_from_hex_str;

    use crate::{
        predeployed_account::PredeployedAccount,
        types::{Felt, Key},
    };

    const PRIVATE_KEYS_IN_HEX: [&str; 3] = [
        "0xc4da537c1651ddae44867db30d67b366",
        "0xd6a82a951b923e0a443cdef36840ff07",
        "0x610e4ad509c47055dff4948fe6b4f832",
    ];

    const SEED: u32 = 123;

    const PUBLIC_KEYS_IN_HEX: [&str; 3] = [
        "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b5",
        "0x5a91f0ea25312accb20d8041b12260bff31a3490e5730a690b0ec8fe10ffbb",
        "0x2b45af00df833ea1a4895c49a18ebd84309b79d658cae05e274a7b1cac47016",
    ];

    /// Test correct generation of private keys
    /// Test expected results are taken from https://github.com/0xSpaceShard/starknet-devnet/blob/master/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn correct_private_key_from_fixed_seed() {
        let predeployed_acc = predeployed_account_instance();
        let generated_private_keys = predeployed_acc.generate_private_keys(3).unwrap();

        let expected_result = PRIVATE_KEYS_IN_HEX
            .into_iter()
            .map(|hex_str| Felt(bytes_from_hex_str::<32, true>(hex_str).unwrap()))
            .collect::<Vec<Felt>>();

        assert_eq!(expected_result, generated_private_keys);

        let string_result =
            generated_private_keys.into_iter().map(|pk| pk.to_prefixed_hex_str()).collect::<Vec<String>>();

        assert_eq!(string_result, PRIVATE_KEYS_IN_HEX.to_vec());
    }

    #[test]
    fn correct_public_key_from_private_key() {
        let predeployed_acc = predeployed_account_instance();
        let generated_private_keys = predeployed_acc.generate_private_keys(3).unwrap();

        let generated_public_keys = generated_private_keys
            .into_iter()
            .map(|pk| predeployed_acc.generate_public_key(&pk).unwrap())
            .collect::<Vec<Key>>();

        let mut expected_public_keys = PUBLIC_KEYS_IN_HEX
            .into_iter()
            .map(|public_key_hex| Felt::from_prefixed_hex_str(public_key_hex).unwrap())
            .collect::<Vec<Key>>();

        assert_eq!(generated_public_keys, expected_public_keys);
    }

    fn predeployed_account_instance() -> PredeployedAccount {
        PredeployedAccount::new(SEED, 100)
    }
}
