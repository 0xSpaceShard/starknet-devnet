use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use starknet_types::contract_class::ContractClass;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use starknet_types::rpc::transactions::{DeclareTransaction, InvokeTransaction, Transaction};

use super::{DumpMode, Starknet};
use crate::error::{DevnetResult, Error};

impl Starknet {
    pub fn re_execute(&mut self, transactions: Vec<Transaction>) -> DevnetResult<()> {
        for transaction in transactions.iter() {
            match transaction {
                Transaction::Declare(DeclareTransaction::Version0(_)) => {
                    return Err(Error::SerializationNotSupported);
                }
                Transaction::Declare(DeclareTransaction::Version1(tx)) => {
                    let contract_class = self
                        .state
                        .contract_classes
                        .get(&tx.class_hash)
                        .ok_or(Error::ContractClassLoadError)?;
                    if let ContractClass::Cairo0(contract) = contract_class {
                        let declare_tx = BroadcastedDeclareTransactionV1::new(
                            tx.sender_address,
                            tx.max_fee,
                            &tx.signature,
                            tx.nonce,
                            contract,
                            tx.version,
                        );
                        self.add_declare_transaction_v1(declare_tx)?;
                    } else {
                        return Err(Error::SerializationNotSupported);
                    };
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
                Transaction::DeployAccount(tx) => {
                    let deploy_account_tx = BroadcastedDeployAccountTransaction::new(
                        &tx.constructor_calldata,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        tx.class_hash,
                        tx.contract_address_salt,
                        tx.version,
                    );
                    self.add_deploy_account_transaction(deploy_account_tx)?;
                }
                Transaction::Deploy(_) => return Err(Error::SerializationNotSupported),
                Transaction::Invoke(InvokeTransaction::Version0(_)) => {
                    return Err(Error::SerializationNotSupported);
                }
                Transaction::Invoke(InvokeTransaction::Version1(tx)) => {
                    let invoke_tx = BroadcastedInvokeTransaction::new(
                        tx.sender_address,
                        tx.max_fee,
                        &tx.signature,
                        tx.nonce,
                        &tx.calldata,
                        tx.version,
                    );
                    self.add_invoke_transaction(invoke_tx)?;
                }
                Transaction::L1Handler(_) => return Err(Error::SerializationNotSupported),
            };
        }

        Ok(())
    }

    /// attach starknet transaction to end of existing file
    pub fn dump_transaction(&self, transaction: &Transaction) -> DevnetResult<()> {
        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);
                if file_path.exists() {
                    // attach to file
                    let transaction_dump = serde_json::to_string(transaction).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    let mut file = OpenOptions::new()
                        .append(true)
                        .write(true)
                        .open(file_path)
                        .map_err(Error::IoError)?;
                    let length = file.seek(SeekFrom::End(0)).map_err(Error::IoError)?;
                    file.set_len(length - 1).map_err(Error::IoError)?; // remove last "]" with set_len
                    file.write_all(format!(", {transaction_dump}]").as_bytes())
                        .map_err(Error::IoError)?;
                } else {
                    // create file
                    let transactions = vec![transaction];
                    let transactions_dump = serde_json::to_string(&transactions).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    fs::write(Path::new(&path), transactions_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    /// save starknet transactions to file
    pub fn dump_transactions(&self) -> DevnetResult<()> {
        match &self.config.dump_path {
            Some(path) => {
                let transactions = &self
                    .transactions
                    .iter()
                    .map(|x| x.1.inner.clone())
                    .collect::<Vec<Transaction>>();

                // dump only if there are transactions to dump
                if !transactions.is_empty() {
                    let transactions_dump = serde_json::to_string(transactions).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    fs::write(Path::new(&path), transactions_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    // load starknet transactions from file
    pub fn load_transactions(&self) -> DevnetResult<Vec<Transaction>> {
        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);

                // load only if the file exists, if dump_path is set but the file doesn't exist it
                // can mean that it's first run with dump_path parameter set to dump, in that case
                // return empty vector
                if file_path.exists() {
                    let mut file = File::open(file_path).map_err(Error::IoError)?;
                    let mut data = String::new();
                    file.read_to_string(&mut data).map_err(Error::IoError)?;
                    let transactions: Vec<Transaction> =
                        serde_json::from_str(&data).map_err(|_| Error::DeserializationError {
                            obj_name: "Vec<Transaction>".to_string(),
                        })?;

                    // to avoid doublets in transaction mode during load, we need to remove the file
                    // because they will be re-executed and saved again
                    if self.config.dump_on == Some(DumpMode::OnTransaction) {
                        fs::remove_file(file_path).map_err(Error::IoError)?;
                    }

                    Ok(transactions)
                } else {
                    Ok(Vec::new())
                }
            }
            None => Err(Error::FormatError),
        }
    }
}
