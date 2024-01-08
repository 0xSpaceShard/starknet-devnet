use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, DeclareTransaction, DeployAccountTransaction, InvokeTransaction,
    L1HandlerTransaction, Transaction,
};

use super::{DumpOn, Starknet};
use crate::error::{DevnetResult, Error};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DumpEvent {
    AddDeclareTransaction(BroadcastedDeclareTransaction),
    AddDeployAccountTransaction(BroadcastedDeployAccountTransaction),
    AddInvokeTransaction(BroadcastedInvokeTransaction),
    CreatedBlock,
    Mint(MintEvent),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MintEvent {
    // address: ContractAddress,
    // amount: u128,
    // erc20_address: ContractAddress
}

impl Starknet {
    // TODO: change from Vec<Transaction> to Vec<DumpEvent>
    pub fn re_execute(&mut self, transactions: Vec<Transaction>) -> DevnetResult<()> {
        for transaction in transactions.into_iter() {
            match transaction {
                Transaction::Declare(DeclareTransaction::Version1(tx)) => {
                    let declare_tx = BroadcastedDeclareTransactionV1::new(
                        tx.sender_address,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        &tx.contract_class,
                        tx.version,
                    );
                    self.add_declare_transaction_v1(declare_tx)?;
                }
                Transaction::Declare(DeclareTransaction::Version2(tx)) => {
                    let declare_tx = BroadcastedDeclareTransactionV2::new(
                        &tx.contract_class,
                        tx.compiled_class_hash,
                        tx.sender_address,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        tx.version,
                    );
                    self.add_declare_transaction_v2(declare_tx)?;
                }
                Transaction::Declare(DeclareTransaction::Version3(tx)) => {
                    let declare_tx: BroadcastedDeclareTransactionV3 = tx.into();
                    self.add_declare_transaction_v3(declare_tx)?;
                }
                Transaction::DeployAccount(DeployAccountTransaction::Version1(tx)) => {
                    let deploy_account_tx = BroadcastedDeployAccountTransactionV1::new(
                        &tx.constructor_calldata,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        tx.class_hash,
                        tx.contract_address_salt,
                        tx.version,
                    );
                    self.add_deploy_account_transaction_v1(deploy_account_tx)?;
                }
                Transaction::DeployAccount(DeployAccountTransaction::Version3(tx)) => {
                    let deploy_account_tx: BroadcastedDeployAccountTransactionV3 = (*tx).into();
                    self.add_deploy_account_transaction_v3(deploy_account_tx)?;
                }
                Transaction::Deploy(_) => {
                    return Err(Error::SerializationNotSupported { obj_name: "Deploy tx".into() });
                }
                Transaction::Invoke(InvokeTransaction::Version1(tx)) => {
                    let invoke_tx = BroadcastedInvokeTransactionV1::new(
                        tx.sender_address,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        &tx.calldata,
                        tx.version,
                    );
                    self.add_invoke_transaction_v1(invoke_tx)?;
                }
                Transaction::Invoke(InvokeTransaction::Version3(tx)) => {
                    let invoke_tx: BroadcastedInvokeTransactionV3 = tx.into();
                    self.add_invoke_transaction_v3(invoke_tx)?;
                }
                Transaction::L1Handler(tx) => {
                    self.add_l1_handler_transaction(L1HandlerTransaction {
                        transaction_hash: tx.transaction_hash,
                        version: tx.version,
                        nonce: tx.nonce,
                        contract_address: tx.contract_address,
                        entry_point_selector: tx.entry_point_selector,
                        calldata: tx.calldata.clone(),
                        paid_fee_on_l1: tx.paid_fee_on_l1,
                    })?;
                }
            };
        }

        Ok(())
    }

    // add starknet dump event
    pub fn handle_dump_event(&mut self, event: DumpEvent) -> DevnetResult<()> {
        match self.config.dump_on {
            Some(dump) => {
                self.dump_events.push(event);

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// attach starknet transaction to end of existing file
    pub fn dump_transaction(&self, transaction: &Transaction) -> DevnetResult<()> {
        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);
                if file_path.exists() {
                    // attach to file
                    let transaction_dump = serde_json::to_string(transaction)
                        .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
                    let mut file = OpenOptions::new()
                        .append(true)
                        .read(true)
                        .open(file_path)
                        .map_err(Error::IoError)?;
                    let mut buffer = [0; 1];
                    file.seek(SeekFrom::End(-1))?;
                    file.read_exact(&mut buffer)?;
                    if String::from_utf8_lossy(&buffer).into_owned() == "]" {
                        // if the last character is "]", remove it and add transaction at the end
                        let length = file.seek(SeekFrom::End(0)).map_err(Error::IoError)?;
                        file.set_len(length - 1).map_err(Error::IoError)?; // remove last "]" with set_len
                        file.write_all(format!(", {transaction_dump}]").as_bytes())
                            .map_err(Error::IoError)?;
                    } else {
                        // if the last character is not "]" it means that it's a wrongly formatted
                        // file
                        return Err(Error::FormatError);
                    }
                } else {
                    // create file
                    let transactions = vec![transaction];
                    let transactions_dump = serde_json::to_string(&transactions)
                        .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
                    fs::write(Path::new(&path), transactions_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    pub fn dump_transactions(&self) -> DevnetResult<()> {
        self.dump_transactions_custom_path(None)
    }

    /// save starknet transactions to file
    pub fn dump_transactions_custom_path(&self, custom_path: Option<String>) -> DevnetResult<()> {
        let dump_path = if custom_path.is_some() { &custom_path } else { &self.config.dump_path };
        match dump_path {
            Some(path) => {
                let transactions = &self
                    .transactions
                    .iter()
                    .map(|x| x.1.inner.clone())
                    .collect::<Vec<Transaction>>();

                // dump only if there are transactions to dump
                if !transactions.is_empty() {
                    let transactions_dump = serde_json::to_string(transactions)
                        .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
                    fs::write(Path::new(&path), transactions_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    pub fn load_transactions(&self) -> DevnetResult<Vec<Transaction>> {
        self.load_transactions_custom_path(None)
    }

    // load starknet transactions from file
    pub fn load_transactions_custom_path(
        &self,
        custom_path: Option<String>,
    ) -> DevnetResult<Vec<Transaction>> {
        let dump_path = if custom_path.is_some() { &custom_path } else { &self.config.dump_path };
        match dump_path {
            Some(path) => {
                let file_path = Path::new(path);

                // load only if the file exists, if config.dump_path is set but the file doesn't
                // exist it means that it's first execution and in that case return an empty vector,
                // in case of load from HTTP endpoint return FileNotFound error
                if file_path.exists() {
                    let file = File::open(file_path).map_err(Error::IoError)?;
                    let transactions: Vec<Transaction> = serde_json::from_reader(file)
                        .map_err(|e| Error::DeserializationError { origin: e.to_string() })?;

                    // to avoid doublets in transaction mode during load, we need to remove the file
                    // because they will be re-executed and saved again
                    if self.config.dump_on == Some(DumpOn::Transaction) {
                        fs::remove_file(file_path).map_err(Error::IoError)?;
                    }

                    Ok(transactions)
                } else {
                    Err(Error::FileNotFound)
                }
            }
            None => Err(Error::FormatError),
        }
    }
}
