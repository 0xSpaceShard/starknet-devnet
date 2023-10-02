use std::fs::{self, File, OpenOptions};
use std::io::{Read, SeekFrom, Seek};
use std::io::Write;
use std::path::Path;

use starknet_types::contract_class::ContractClass;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
use starknet_types::rpc::transactions::{DeclareTransaction, InvokeTransaction, Transaction};

use super::Starknet;
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
        println!("re_execute");
        Ok(())
    }

    /// attach starknet transaction to end of existing file
    pub fn dump_transaction(&self, transaction: &Transaction) -> DevnetResult<()> {
        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);
                if file_path.exists() {
                    // TODO append

                    // let mut file = File::open(file_path)?;
                    // let mut v: Vec<u8> = Vec::new();
                    // file.read_to_end(&mut v)?;
                    // let decoded: Result<String, Error> = bincode::deserialize(&v).map_err(|_| {
                    //     Error::DeserializationError { obj_name: "Vec<Transaction>".to_string() }
                    // });
                    // let transactions: DevnetResult<Vec<Transaction>, Error> =
                    //     serde_json::from_str(decoded.unwrap().as_str()).map_err(|_| {
                    //         Error::DeserializationError { obj_name: "Vec<Transaction>".to_string() }
                    //     });

                    let transaction_dump = serde_json::to_string(transaction).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    let encoded: Vec<u8> = bincode::serialize(&transaction_dump).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    let mut file = OpenOptions::new().append(true).write(true).open("dump").expect("TODO 1");
                    let lenght = file.seek(SeekFrom::End(0));
                    print!("lenght: {:?}", lenght);
                    file.set_len(lenght.unwrap() - 1);
                    file.write_all(", ".as_bytes()).expect("TODO failed 2");
                    file.write_all(&encoded).expect("TODO failed 2");
                    file.write_all("]".as_bytes()).expect("TODO failed 3");
                } else {
                    // create file scenario
                    let mut transactions = Vec::new();
                    transactions.push(transaction);
                    let starknet_dump = serde_json::to_string(&transactions).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    let encoded: Vec<u8> = bincode::serialize(&starknet_dump).map_err(|_| {
                        Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                    })?;
                    fs::write(Path::new(&path), encoded)?;
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
                let starknet_dump = serde_json::to_string(transactions).map_err(|_| {
                    Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                })?;
                let encoded: Vec<u8> = bincode::serialize(&starknet_dump).map_err(|_| {
                    Error::SerializationError { obj_name: "Vec<Transaction>".to_string() }
                })?;
                fs::write(Path::new(&path), encoded)?;

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    // load starknet transactions from file
    pub fn load_transactions(&self) -> DevnetResult<Vec<Transaction>> {
        let mut file = OpenOptions::new().append(true).write(true).open("dump").expect("TODO 1");
        let lenght = file.seek(SeekFrom::End(0));
        println!("lenght: {:?}", lenght);
        let x = file.set_len(1);
        print!("x: {:?}", x);
        let lenght = file.seek(SeekFrom::End(0));
        println!("lenght: {:?}", lenght);

        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);

                // load only if the file exists, if dump_path is set but the file doesn't exist it
                // can mean that it's first run with dump_path parameter set to dump, in that case
                // return empty vector
                if file_path.exists() {
                    let mut file = File::open(file_path)?;
                    let mut v: Vec<u8> = Vec::new();
                    file.read_to_end(&mut v)?;
                    let decoded: Result<String, Error> = bincode::deserialize(&v).map_err(|_| {
                        Error::DeserializationError { obj_name: "Vec<Transaction> 123".to_string() }
                    });
                    let transactions: DevnetResult<Vec<Transaction>, Error> =
                        serde_json::from_str(decoded.unwrap().as_str()).map_err(|_| {
                            Error::DeserializationError { obj_name: "Vec<Transaction>".to_string() }
                        });

                    transactions
                } else {
                    Ok(Vec::new())
                }
            }
            None => Err(Error::FormatError),
        }
    }
}
