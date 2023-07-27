use starknet_api::block::BlockNumber;
use starknet_in_rust::felt::Felt252;
use starknet_in_rust::utils::Address;
use starknet_rs_core::types::BlockId;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;

use super::Starknet;
use crate::error::{self, Result};
use crate::traits::HashIdentified;

pub struct EmittedEvent {
    pub transaction_hash: starknet_types::felt::TransactionHash,
    pub block_hash: starknet_types::felt::BlockHash,
    pub block_number: BlockNumber,
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}

pub(crate) fn get_events(
    starknet: &Starknet,
    from_block: Option<BlockId>,
    to_block: Option<BlockId>,
    contract_address: Option<ContractAddress>,
    keys_filter: Option<Vec<Vec<Felt>>>
) -> Result<Vec<EmittedEvent>> {
    let blocks = starknet.blocks.get_blocks(from_block, to_block)?;
    let mut events: Vec<EmittedEvent> = Vec::new();
    // convert to starknet_in_rust::utils::Address
    let address =
        if let Some(address) = contract_address { Some(Address::try_from(address)?) } else { None };
    // convert felts to Felt252
    let keys_filter: Option<Vec<Vec<Felt252>>> = keys_filter.map(|felts| {
        felts
            .into_iter()
            .map(|inner_felts| inner_felts.into_iter().map(|felt| Felt252::from(felt)).collect())
            .collect()
    });

    // iterate over each block and get the transactions for each one
    // then iterate over each transaction events and filter them
    for block in blocks {
        for transaction_hash in block.get_transactions() {
            let transaction = starknet
                .transactions
                .get_by_hash(*transaction_hash)
                .ok_or(crate::error::Error::NoTransaction)?;

            // filter the events from the transaction
            let filtered_transaction_events =
                transaction.get_events()?.into_iter().filter(|event| {
                    let address_condition = match &address {
                        Some(from_contract_address) => {
                            event.from_address == from_contract_address.clone()
                        }
                        None => true,
                    };

                    // address condition is false, then no need to continue checking the keys
                    if !address_condition {
                        return false;
                    }

                    match &keys_filter {
                        Some(keys_filter) => {
                            for (event_key, accepted_keys) in event.keys.iter().zip(keys_filter) {
                                if accepted_keys.len() > 0 && !accepted_keys.contains(event_key) {
                                    return false;
                                }
                            }

                            return true;
                        }
                        None => true,
                    }
                });

            // produce an emitted event for each filtered transaction event
            for transaction_event in filtered_transaction_events {
                events.push(EmittedEvent {
                    transaction_hash: *transaction_hash,
                    block_hash: block.block_hash(),
                    block_number: block.block_number(),
                    from_address: transaction_event
                        .from_address
                        .try_into()
                        .map_err(error::Error::from)?,
                    keys: transaction_event.keys.into_iter().map(|el| el.into()).collect(),
                    data: transaction_event.data.into_iter().map(|el| el.into()).collect(),
                });
            }
        }
    }

    Ok(events)
}
