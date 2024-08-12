use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use starknet_core::error::{DevnetResult, Error};
use starknet_core::starknet::starknet_config::DumpOn;

use crate::rpc_core::request::RpcMethodCall;

pub type DumpEvent = RpcMethodCall;

/// Saves Devnet `events` to the file at `path`. If `events` is empty, does nothing.
pub fn dump_events(events: &Vec<DumpEvent>, path: &str) -> DevnetResult<()> {
    if !events.is_empty() {
        let events_dump = serde_json::to_string(events)
            .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
        fs::write(Path::new(&path), events_dump)?;
    }

    Ok(())
}

/// Attaches starknet event to the end of the file at `path`. If no file present, creates it.
pub fn dump_event(event: &DumpEvent, path: &str) -> DevnetResult<()> {
    let file_path = Path::new(path);
    if file_path.exists() {
        // attach to file
        let event_dump = serde_json::to_string(event)
            .map_err(|e| Error::SerializationError { origin: e.to_string() })?;
        let mut file =
            OpenOptions::new().append(true).read(true).open(file_path).map_err(Error::IoError)?;
        let mut buffer = [0; 1];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut buffer)?;
        if String::from_utf8_lossy(&buffer).into_owned() == "]" {
            // if the last character is "]", remove it and add event at the end
            let length = file.seek(SeekFrom::End(0)).map_err(Error::IoError)?;
            file.set_len(length - 1).map_err(Error::IoError)?; // remove last "]" with set_len
            file.write_all(format!(", {event_dump}]").as_bytes()).map_err(Error::IoError)?;
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

/// Returns Devnet events from the provided `path`
pub fn load_events(dump_on: Option<DumpOn>, path: &str) -> DevnetResult<Vec<DumpEvent>> {
    let file_path = Path::new(path);
    if path.is_empty() || !file_path.exists() {
        return Err(Error::FileNotFound);
    }

    let file = File::open(file_path).map_err(Error::IoError)?;
    let events: Vec<DumpEvent> = serde_json::from_reader(file)
        .map_err(|e| Error::DeserializationError { origin: e.to_string() })?;

    // to avoid doublets in block mode during load, we need to remove the file
    // because they will be re-executed and saved again
    if dump_on == Some(DumpOn::Block) {
        // TODO shouldn't this be the responsibility of this method
        fs::remove_file(file_path).map_err(Error::IoError)?;
    }

    Ok(events)
}
