use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, L1HandlerTransaction,
};

use super::{DumpOn, Starknet};
use crate::error::{DevnetResult, Error};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DumpEvent {
    CreateBlock,
    SetTime(u64),
    SetTimeCreateBlock(u64),
    IncreaseTime(u64),
    AddDeclareTransaction(BroadcastedDeclareTransaction),
    AddInvokeTransaction(BroadcastedInvokeTransaction),
    AddDeployAccountTransaction(BroadcastedDeployAccountTransaction),
    AddL1HandlerTransaction(L1HandlerTransaction),
}

impl Starknet {
    pub fn re_execute(&mut self, events: Vec<DumpEvent>) -> DevnetResult<()> {
        for event in events.into_iter() {
            match event {
                DumpEvent::AddDeclareTransaction(BroadcastedDeclareTransaction::V1(tx)) => {
                    self.add_declare_transaction_v1(*tx)?;
                }
                DumpEvent::AddDeclareTransaction(BroadcastedDeclareTransaction::V2(tx)) => {
                    self.add_declare_transaction_v2(*tx)?;
                }
                DumpEvent::AddDeclareTransaction(BroadcastedDeclareTransaction::V3(tx)) => {
                    self.add_declare_transaction_v3(*tx)?;
                }
                DumpEvent::AddDeployAccountTransaction(
                    BroadcastedDeployAccountTransaction::V1(tx),
                ) => {
                    self.add_deploy_account_transaction_v1(tx)?;
                }
                DumpEvent::AddDeployAccountTransaction(
                    BroadcastedDeployAccountTransaction::V3(tx),
                ) => {
                    self.add_deploy_account_transaction_v3(tx)?;
                }
                DumpEvent::AddInvokeTransaction(BroadcastedInvokeTransaction::V1(tx)) => {
                    self.add_invoke_transaction_v1(tx)?;
                }
                DumpEvent::AddInvokeTransaction(BroadcastedInvokeTransaction::V3(tx)) => {
                    self.add_invoke_transaction_v3(tx)?;
                }
                DumpEvent::AddL1HandlerTransaction(tx) => {
                    self.add_l1_handler_transaction(tx)?;
                }
                DumpEvent::CreateBlock => {
                    self.create_block_dump_event(None)?;
                }
                DumpEvent::SetTime(timestamp) => {
                    self.set_time(timestamp, false)?;
                }
                DumpEvent::SetTimeCreateBlock(timestamp) => {
                    self.set_time(timestamp, true)?;
                }
                DumpEvent::IncreaseTime(time_shift) => {
                    self.increase_time(time_shift)?;
                }
            };
        }

        Ok(())
    }

    // add starknet dump event
    pub fn handle_dump_event(&mut self, event: DumpEvent) -> DevnetResult<()> {
        match self.config.dump_on {
            Some(DumpOn::Transaction) => self.dump_event(event),
            Some(DumpOn::Exit) => {
                self.dump_events.push(event);

                Ok(())
            }
            None => Ok(()),
        }
    }

    /// attach starknet event to end of existing file
    pub fn dump_event(&self, event: DumpEvent) -> DevnetResult<()> {
        match &self.config.dump_path {
            Some(path) => {
                let file_path = Path::new(path);
                if file_path.exists() {
                    // attach to file
                    let event_dump = serde_json::to_string(&event)
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
                        // if the last character is "]", remove it and add event at the end
                        let length = file.seek(SeekFrom::End(0)).map_err(Error::IoError)?;
                        file.set_len(length - 1).map_err(Error::IoError)?; // remove last "]" with set_len
                        file.write_all(format!(", {event_dump}]").as_bytes())
                            .map_err(Error::IoError)?;
                    } else {
                        // if the last character is not "]" it means that it's a wrongly formatted
                        // file
                        return Err(Error::FormatError);
                    }
                } else {
                    // create file
                    let events = vec![event];
                    let events_dump = serde_json::to_string(&events)
                        .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
                    fs::write(Path::new(&path), events_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    pub fn dump_events(&self) -> DevnetResult<()> {
        self.dump_events_custom_path(None)
    }

    /// save starknet events to file
    pub fn dump_events_custom_path(&self, custom_path: Option<String>) -> DevnetResult<()> {
        let dump_path = if custom_path.is_some() { &custom_path } else { &self.config.dump_path };
        match dump_path {
            Some(path) => {
                let events = &self.dump_events;

                // dump only if there are events to dump
                if !events.is_empty() {
                    let events_dump = serde_json::to_string(events)
                        .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
                    fs::write(Path::new(&path), events_dump)?;
                }

                Ok(())
            }
            None => Err(Error::FormatError),
        }
    }

    pub fn load_events(&self) -> DevnetResult<Vec<DumpEvent>> {
        self.load_events_custom_path(None)
    }

    // load starknet events from file
    pub fn load_events_custom_path(
        &self,
        custom_path: Option<String>,
    ) -> DevnetResult<Vec<DumpEvent>> {
        let dump_path = if custom_path.is_some() { &custom_path } else { &self.config.dump_path };
        match dump_path {
            Some(path) => {
                let file_path = Path::new(path);

                // load only if the file exists, if config.dump_path is set but the file doesn't
                // exist it means that it's first execution and in that case return an empty vector,
                // in case of load from HTTP endpoint return FileNotFound error
                if file_path.exists() {
                    let file = File::open(file_path).map_err(Error::IoError)?;
                    let events: Vec<DumpEvent> = serde_json::from_reader(file)
                        .map_err(|e| Error::DeserializationError { origin: e.to_string() })?;

                    // to avoid doublets in transaction mode during load, we need to remove the file
                    // because they will be re-executed and saved again
                    if self.config.dump_on == Some(DumpOn::Transaction) {
                        fs::remove_file(file_path).map_err(Error::IoError)?;
                    }

                    Ok(events)
                } else {
                    Err(Error::FileNotFound)
                }
            }
            None => Err(Error::FormatError),
        }
    }
}
