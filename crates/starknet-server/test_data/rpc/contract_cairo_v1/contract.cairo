#[contract]
mod Contract {
    struct Storage {
        balance: felt252,
    }

    #[constructor]
    fn constructor(initial_balance: felt252) {
        balance::write(initial_balance);
    }

    #[external]
    fn increase_balance(amount1: felt252, amount2: felt252) {
        balance::write(balance::read() + amount1 + amount2);
    }

    #[view]
    fn get_balance() -> felt252 {
        balance::read()
    }
}