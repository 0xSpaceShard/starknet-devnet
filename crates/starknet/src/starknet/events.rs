use starknet_api::block::BlockNumber;
use starknet_in_rust::execution::Event;
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

/// The method returns transaction events, based on query and if there are more results to be
/// fetched
///
/// # Arguments
///
/// * `from_block` - Optional. The block id to start the query from.
/// * `to_block` - Optional. The block id to end the query at.
/// * `contract_address` - Optional. The contract address to filter the events by.
/// * `keys_filter` - Optional. The keys to filter the events by.
/// * `skip` - The number of elements to skip.
/// * `limit` - Optional. The maximum number of elements to return.
pub(crate) fn get_events(
    starknet: &Starknet,
    from_block: Option<BlockId>,
    to_block: Option<BlockId>,
    contract_address: Option<ContractAddress>,
    keys_filter: Option<Vec<Vec<Felt>>>,
    mut skip: usize,
    limit: Option<usize>,
) -> Result<(Vec<EmittedEvent>, bool)> {
    let blocks = starknet.blocks.get_blocks(from_block, to_block)?;
    let mut events: Vec<EmittedEvent> = Vec::new();
    // convert to starknet_in_rust::utils::Address
    let address =
        if let Some(address) = contract_address { Some(Address::try_from(address)?) } else { None };
    // convert felts to Felt252
    let keys_filter: Option<Vec<Vec<Felt252>>> = keys_filter.map(|felts| {
        felts
            .into_iter()
            .map(|inner_felts| inner_felts.into_iter().map(Felt252::from).collect())
            .collect()
    });
    let mut elements_added = 0;

    // iterate over each block and get the transactions for each one
    // then iterate over each transaction events and filter them
    for block in blocks {
        for transaction_hash in block.get_transactions() {
            let transaction = starknet
                .transactions
                .get_by_hash(*transaction_hash)
                .ok_or(crate::error::Error::NoTransaction)?;

            // filter the events from the transaction
            let filtered_transaction_events = transaction
                .get_events()?
                .into_iter()
                .filter(|event| check_if_filter_applies_for_event(&address, &keys_filter, event));

            // produce an emitted event for each filtered transaction event
            for transaction_event in filtered_transaction_events.skip(skip) {
                // check if there are more elements to fetch
                if let Some(limit) = limit {
                    if elements_added == limit {
                        return Ok((events, true));
                    }
                }

                let emitted_event = EmittedEvent {
                    transaction_hash: *transaction_hash,
                    block_hash: block.block_hash(),
                    block_number: block.block_number(),
                    from_address: transaction_event
                        .from_address
                        .try_into()
                        .map_err(error::Error::from)?,
                    keys: transaction_event.keys.into_iter().map(|el| el.into()).collect(),
                    data: transaction_event.data.into_iter().map(|el| el.into()).collect(),
                };

                events.push(emitted_event);
                elements_added += 1;
            }

            // modify how many elements to skip, whichever is smaller so the usize doens't overflow
            skip = skip - std::cmp::min(skip, elements_added);
        }
    }

    Ok((events, false))
}

/// This method checks if the event applies to the provided filters and returns true or false
///
/// # Arguments
/// * `address` - Optional. The address to filter the event by.
/// * `keys_filter` - Optional. The keys to filter the event by.
/// * `event` - The event to check if it applies to the filters.
fn check_if_filter_applies_for_event(
    address: &Option<starknet_in_rust::utils::Address>,
    keys_filter: &Option<Vec<Vec<Felt252>>>,
    event: &Event,
) -> bool {
    let address_condition = match &address {
        Some(from_contract_address) => event.from_address == from_contract_address.clone(),
        None => true,
    };

    address_condition && check_if_filter_applies_for_event_keys(keys_filter, &event.keys)
}

/// This method checks if the keys apply to the keys_filter and returns true or false
///
/// # Arguments
/// * `keys_filter` - Optional. The values to filter the keys by.
/// * `keys` - The keys to check if they apply to the filter.
fn check_if_filter_applies_for_event_keys<T>(
    keys_filter: &Option<Vec<Vec<T>>>,
    keys: &[T],
) -> bool
where
    T: PartialEq + Eq,
{
    match &keys_filter {
        Some(keys_filter) => {
            for (event_key, accepted_keys) in keys.iter().zip(keys_filter) {
                if !accepted_keys.is_empty() && !accepted_keys.contains(event_key) {
                    return false;
                }
            }

            true
        }
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::{Event, OrderedEvent};
    use starknet_in_rust::felt::Felt252;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;

    use super::check_if_filter_applies_for_event;
    use crate::starknet::events::check_if_filter_applies_for_event_keys;
    use crate::utils::test_utils::dummy_contract_address;

    #[test]
    fn filter_keys_with_empty_or_no_filter() {
        let keys = vec![1u32];
        // no filter
        assert!(check_if_filter_applies_for_event_keys(&None, &keys));

        // empty filter
        let filter = vec![];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // empty filter, but made of two empty filters
        let filter = vec![vec![], vec![]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));
    }

    #[test]
    fn filter_applies_to_single_key() {
        // check for 1 key
        let keys = vec![1u32];

        // filter with 1 key and second one empty filter
        let filter = vec![vec![1u32], vec![]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // filter with 1 key and second one value that is not amongst the keys, but will not
        // evalueate, because the keys is of length 1
        let filter = vec![vec![1u32], vec![2u32]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // filter with multiple keys, that are different from the keys, except one and second filter
        // is empty
        let filter = vec![vec![0u32, 1u32], vec![]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));
    }

    #[test]
    fn filter_does_not_apply_to_single_key() {
        let keys = vec![1u32];

        // filter with 1 key, that is different from the keys and second one empty filter
        let filter = vec![vec![0u32], vec![]];
        assert!(!check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // filter with multiple keys, that are different from the keys and second filter is empty
        let filter = vec![vec![0u32, 2u32], vec![]];
        assert!(!check_if_filter_applies_for_event_keys(&Some(filter), &keys));
    }

    #[test]
    fn filter_applies_to_multiple_keys() {
        let keys = vec![3u32, 2u32];

        // both filters apply to the keys, each filter is with 1 value
        let filter = vec![vec![3u32], vec![2u32]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // both filter apply to the keys, each filter is with multiple values
        let filter = vec![vec![3u32, 1u32], vec![0u32, 2u32]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // first filter applies to the keys, second filter is empty
        let filter = vec![vec![3u32, 1u32], vec![]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // first filter is empty, second filter applies to the keys
        let filter = vec![vec![], vec![0u32, 2u32]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // both filters are empty
        let filter = vec![vec![], vec![]];
        assert!(check_if_filter_applies_for_event_keys(&Some(filter), &keys));
    }

    #[test]
    fn filter_does_not_apply_to_multiple_keys() {
        let keys = vec![3u32, 2u32];

        // first filter applies to the keys, second filter does not
        let filter = vec![vec![3u32, 1u32], vec![0u32]];
        assert!(!check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // first filter does not apply to the keys, second filter applies
        let filter = vec![vec![0u32], vec![0u32, 2u32]];
        assert!(!check_if_filter_applies_for_event_keys(&Some(filter), &keys));

        // both filters do not apply to the keys
        let filter = vec![vec![0u32], vec![0u32]];
        assert!(!check_if_filter_applies_for_event_keys(&Some(filter), &keys));
    }

    #[test]
    fn filter_with_address_only() {
        let event = setup_event();

        // filter with address that is the same as the on in the event
        let address = Some(dummy_contract_address().try_into().unwrap());
        assert!(check_if_filter_applies_for_event(&address, &None, &event));

        // filter with address that is different from the one in the event
        let address = ContractAddress::new(Felt::from(0)).unwrap().try_into().unwrap();
        assert!(!check_if_filter_applies_for_event(&Some(address), &None, &event));
    }

    #[test]
    fn filter_with_keys_only() {
        let event = setup_event();

        let keys_filter = vec![vec![Felt252::from(1), Felt252::from(3)]];
        assert!(!check_if_filter_applies_for_event(&None, &Some(keys_filter), &event));

        let keys_filter = vec![vec![], vec![Felt252::from(1), Felt252::from(3)]];
        assert!(check_if_filter_applies_for_event(&None, &Some(keys_filter), &event));
    }

    #[test]
    fn filter_with_address_and_keys() {
        let event = setup_event();

        // filter with address correct and filter keys correct
        let address = Some(dummy_contract_address().try_into().unwrap());
        let keys_filter = vec![vec![Felt252::from(2), Felt252::from(3)]];
        assert!(check_if_filter_applies_for_event(&address, &Some(keys_filter), &event));

        // filter with incorrect address and correct filter keys
        let address = Some(ContractAddress::new(Felt::from(0)).unwrap().try_into().unwrap());
        let keys_filter = vec![vec![Felt252::from(2), Felt252::from(3)]];
        assert!(!check_if_filter_applies_for_event(&address, &Some(keys_filter), &event));

        // filter with correct address and incorrect filter keys
        let address = Some(dummy_contract_address().try_into().unwrap());
        let keys_filter = vec![vec![Felt252::from(1), Felt252::from(3)]];
        assert!(!check_if_filter_applies_for_event(&address, &Some(keys_filter), &event));

        // filter with incorrect address and incorrect filter keys
        let address = Some(ContractAddress::new(Felt::from(0)).unwrap().try_into().unwrap());
        let keys_filter = vec![vec![Felt252::from(1), Felt252::from(3)]];
        assert!(!check_if_filter_applies_for_event(&address, &Some(keys_filter), &event));
    }

    fn setup_event() -> Event {
        Event::new(
            OrderedEvent::new(
                1,
                vec![Felt252::from(2), Felt252::from(3)],
                vec![Felt252::from(1), Felt252::from(1)],
            ),
            dummy_contract_address().try_into().unwrap(),
        )
    }
}
