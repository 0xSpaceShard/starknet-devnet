use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction,
};

use super::starknet_config::StarknetConfig;
use super::{DumpOn, Starknet};
use crate::error::{DevnetResult, Error};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DumpEvent {
    CreateBlock,
    SetTime(u64),
    IncreaseTime(u64),
    AddDeclareTransaction(BroadcastedDeclareTransaction),
    AddInvokeTransaction(BroadcastedInvokeTransaction),
    AddDeployAccountTransaction(BroadcastedDeployAccountTransaction),
    AddL1HandlerTransaction(L1HandlerTransaction),
}

impl Starknet {
    /// Create an instance of Starknet with the state generated from the events loaded from `path`.
    pub fn load(config: &StarknetConfig, path: &str) -> DevnetResult<Self> {
        let mut this = Self::new(config)?;

        // Try to load events from the path. Since the same CLI parameter is used for dump and load
        // path, it may be the case that there is no file at the path. This means that the file will
        // be created during Devnet's lifetime via dumping, so its non-existence is here ignored.
        match this.load_events(path) {
            Ok(events) => this.re_execute(events)?,
            Err(Error::FileNotFound) => {}
            Err(err) => return Err(err),
        };

        Ok(this)
    }

    pub fn re_execute(&mut self, events: Vec<DumpEvent>) -> DevnetResult<()> {
        for event in events.into_iter() {
            match event {
                DumpEvent::AddDeclareTransaction(tx) => {
                    self.add_declare_transaction(tx)?;
                }
                DumpEvent::AddDeployAccountTransaction(tx) => {
                    self.add_deploy_account_transaction(tx)?;
                }
                DumpEvent::AddInvokeTransaction(tx) => {
                    self.add_invoke_transaction(tx)?;
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
            Some(DumpOn::Block) => self.dump_event(event),
            Some(DumpOn::Request | DumpOn::Exit) => {
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

    pub fn read_dump_events(&self) -> &Vec<DumpEvent> {
        &self.dump_events
    }

    /// Returns Devnet events from the provided `path`
    pub fn load_events(&self, path: &str) -> DevnetResult<Vec<DumpEvent>> {
        let file_path = Path::new(path);
        if path.is_empty() || !file_path.exists() {
            return Err(Error::FileNotFound);
        }

        let file = File::open(file_path).map_err(Error::IoError)?;
        let events: Vec<DumpEvent> = serde_json::from_reader(file)
            .map_err(|e| Error::DeserializationError { origin: e.to_string() })?;

        // to avoid doublets in block mode during load, we need to remove the file
        // because they will be re-executed and saved again
        if self.config.dump_on == Some(DumpOn::Block) {
            fs::remove_file(file_path).map_err(Error::IoError)?;
        }

        Ok(events)
    }
}
