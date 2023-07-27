// use starknet_rs_core::types::BlockId;
// use starknet_types::contract_address::ContractAddress;
// use starknet_types::felt::Felt;

// use super::Starknet;
// use crate::error::Result;

// pub(crate) fn get_events(
//     starknet: &Starknet,
//     from_block: Option<BlockId>,
//     to_block: Option<BlockId>,
//     address: ContractAddress,
//     keys: Vec<Felt>,
// ) -> Result<()> {
//     let blocks = starknet.blocks.get_blocks(from_block, to_block)?;
//     for block in blocks {
//         for transaction in block.get_transactions() {

//             // let events = transaction.get_events();
//             // for event in events {
//             //     if event.contract_address == address {
//             //         let event_data = event.get_data();
//             //         for key in keys {
//             //             if event_data.contains_key(&key) {
//             //                 println!("{}: {}", key, event_data.get(&key).unwrap());
//             //             }
//             //         }
//             //     }
//             // }
//         }
//     }
//     Ok(())
// }
