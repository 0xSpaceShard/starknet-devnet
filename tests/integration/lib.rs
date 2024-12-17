#![cfg(test)]

// TODO need to use cfg or not?

mod common;

mod general_integration_tests;
mod general_rpc_tests;
mod get_transaction_by_block_id_and_index;
mod get_transaction_by_hash;
mod get_transaction_receipt_by_hash;
mod test_abort_blocks;
mod test_account_impersonation;
mod test_account_selection;
mod test_advancing_time;
mod test_balance;
mod test_blocks_generation;
mod test_call;
mod test_dump_and_load;
mod test_estimate_fee;
mod test_estimate_message_fee;
mod test_fork;
mod test_gas_modification;
mod test_get_block_txs_count;
mod test_get_class;
mod test_get_class_hash_at;
mod test_get_events;
mod test_messaging;
mod test_minting;
mod test_old_state;
mod test_restart;
mod test_restrictive_mode;
mod test_simulate_transactions;
mod test_trace;
mod test_transaction_handling;
mod test_v3_transactions;
