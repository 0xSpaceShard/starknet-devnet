use std::{ffi::OsStr, io::ErrorKind, path::Path};

use lazy_static::lazy_static;
use regex::Regex;
use reqwest::StatusCode;
use serde_json::Value;
use starknet_core::error::{DevnetResult, Error};
use starknet_rs_core::types::Felt;
use url::Url;
use walkdir::WalkDir;

use crate::api::{
    http::models::{ContractSource, LoadPath},
    json_rpc::error::{ApiError, DebuggingError},
};

lazy_static! {
    static ref REGEX: Regex =
        Regex::new(r"#\[\s*starknet::contract(?:\(account\))?\s*\]\s*mod (\w+)\b").unwrap();
    static ref ALLOWED_EXTENSIONS: [&'static OsStr; 2] = [OsStr::new("cairo"), OsStr::new("toml")];
}

const WALNUT_VERIFICATION_URL: &str = "https://api.walnut.dev/v1/verify";
const WALNUT_TRANSACTION_DEBUGGING_URL: &str = "https://app.walnut.dev/transactions";

#[derive(Clone)]
pub struct WalnutClient {
    reqwest_client: reqwest::Client,
    ngrok_url: String,
    walnut_api_key: String,
}

impl WalnutClient {
    pub fn new(ngrok_url: String, walnut_api_key: String) -> Self {
        Self { reqwest_client: reqwest::Client::new(), ngrok_url, walnut_api_key }
    }

    pub fn get_url_for_debugging(&self, transaction_hash: Felt) -> Result<String, DebuggingError> {
        Ok(Url::parse_with_params(
            WALNUT_TRANSACTION_DEBUGGING_URL,
            &[
                ("rpcUrl", self.ngrok_url.as_str()),
                ("txHash", transaction_hash.to_fixed_hex_string().as_str()),
            ],
        )
        .map_err(|err| DebuggingError::Custom { error: err.to_string() })?
        .to_string())
    }

    pub async fn verify(
        &self,
        contract_names: Vec<String>,
        class_hashes: Vec<Felt>,
        source_code: serde_json::Value,
    ) -> Result<String, DebuggingError> {
        let json_payload = serde_json::json!( {
            "class_names": contract_names,
            "class_hashes": class_hashes,
            "rpc_url": self.ngrok_url,
            "source_code": source_code,
        });

        let response = self
            .reqwest_client
            .post(WALNUT_VERIFICATION_URL)
            .header("Content-Type", "application/json")
            .header("x-api-key", self.walnut_api_key.as_str())
            .body(json_payload.to_string())
            .send()
            .await
            .map_err(|err| DebuggingError::WalnutProviderError { error: err.to_string() })?;

        // check if response status is 400 and the error is  This class is already verified, then its ok.
        let status = response.status();
        let response_txt = response.text().await.unwrap_or_default();

        match status {
            StatusCode::OK => Ok(response_txt),
            StatusCode::BAD_REQUEST if response_txt.contains("This class is already verified") => {
                Ok(response_txt)
            }
            status_code => Err(DebuggingError::WalnutProviderError {
                error: format!("{} {}", status_code, response_txt),
            }),
        }
    }
}

pub(crate) struct File {
    pub(crate) file_name: String,
    pub(crate) content: String,
}

pub fn get_contract_names<'a, I>(file_contents: I) -> Vec<String>
where
    I: Iterator<Item = &'a serde_json::Value>,
{
    let contract_names = file_contents
        .filter_map(|v| v.as_str().and_then(|s| get_first_word_using_regex(s, &REGEX)))
        .collect::<Vec<String>>();

    contract_names
}

pub(crate) async fn get_cairo_and_toml_files_from_contract_source_in_json_format(
    contract_source: ContractSource,
) -> Result<serde_json::Map<String, Value>, ApiError> {
    let file_contents = match contract_source {
        ContractSource::Path(LoadPath { path: workspace_dir }) => {
            get_cairo_and_toml_files_from_directory(&workspace_dir)
                .await?
                .into_iter()
                .map(|f| (f.file_name, serde_json::Value::String(f.content)))
                .into_iter()
                .collect::<serde_json::Map<String, serde_json::Value>>()
        }
        // Mapping entries are expected to be in the form: (<filename + extension>, <content>)
        ContractSource::Files(mut file_contents) => {
            file_contents.retain(|key, _| {
                let path = Path::new(key);
                if let Some(extension) = path.extension() {
                    ALLOWED_EXTENSIONS.contains(&extension)
                } else {
                    false
                }
            });
            file_contents
        }
    };

    if file_contents.is_empty() {
        return Err(ApiError::from(DebuggingError::SmartContractFilesNotProvided));
    }

    Ok(file_contents)
}

fn get_first_word_using_regex(data: &str, regex: &Regex) -> Option<String> {
    if let Some(captures) = regex.captures(data) {
        if let Some(word) = captures.get(1) {
            return Some(word.as_str().to_string());
        }
    }

    None
}

pub(crate) async fn get_cairo_and_toml_files_from_directory(
    workspace_dir: &str,
) -> DevnetResult<Vec<File>> {
    let mut result = vec![];
    // Recursively read files and their contents in workspace directory
    for entry in WalkDir::new(workspace_dir).follow_links(true) {
        let entry = entry.map_err(|err| {
            let io_error = if let Some(io_error) = err.into_io_error() {
                io_error
            } else {
                std::io::Error::new(ErrorKind::Other, "Filesystem loop due to symlink")
            };

            Error::IoError(io_error)
        })?;

        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if ALLOWED_EXTENSIONS.contains(&extension) {
                    // Unwrapping here is safe, because we already traversed the directory
                    let file_name = path
                        .strip_prefix(workspace_dir)
                        .unwrap()
                        .to_str()
                        .ok_or(Error::UnsupportedAction {
                            msg: "non-unicode characters in the path are not supported".to_string(),
                        })?
                        .to_string();
                    let file_content = tokio::fs::read_to_string(path).await?;

                    result.push(File { file_name, content: file_content });
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        api::http::models::ContractSource,
        walnut_util::{
            get_cairo_and_toml_files_from_contract_source_in_json_format,
            get_cairo_and_toml_files_from_directory, get_first_word_using_regex, REGEX,
        },
    };

    #[test]
    fn test_is_filename_correctly_converted_to_path() {
        let a = Path::new("lib.cairo");
        assert_eq!(a.extension().unwrap(), "cairo");
    }

    #[test]
    fn test_get_first_word_using_regex() {
        let data = "#[starknet::contract] mod my_contract;";
        let result = get_first_word_using_regex(data, &REGEX);
        assert_eq!(result, Some("my_contract".to_string()));
    }

    #[tokio::test]
    async fn test_extract_cairo_and_toml_files_from_directory() {
        let temp_dir = tempfile::tempdir_in("./").unwrap();
        let cairo_file = temp_dir.path().join("lib.cairo");
        let txt_file = temp_dir.path().join("dummy.txt");

        tokio::fs::write(cairo_file, "some cairo stuff").await.unwrap();
        tokio::fs::write(txt_file, "some cairo stuff").await.unwrap();

        let files = get_cairo_and_toml_files_from_directory(temp_dir.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "lib.cairo");
    }

    #[tokio::test]
    async fn test_remove_non_cairo_or_toml_entries_from_mapping() {
        let json_obj = serde_json::json!({
            "file.cairo": "x",
            "file.toml": "y",
            "file.txt": "z"
        });

        let json_obj = get_cairo_and_toml_files_from_contract_source_in_json_format(
            ContractSource::Files(json_obj.as_object().unwrap().to_owned()),
        )
        .await
        .unwrap();

        assert_eq!(json_obj.len(), 2);
        assert!(json_obj.contains_key("file.cairo"));
        assert!(json_obj.contains_key("file.toml"));
        assert!(!json_obj.contains_key("file.txt"));
    }
}
