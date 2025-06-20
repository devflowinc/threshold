use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub enum NodeError {
    Error(String),
}

#[derive(Debug)]
pub enum NetworkError {
    SendError(String),
    RecvError,
}

impl From<rocksdb::Error> for NodeError {
    fn from(e: rocksdb::Error) -> Self {
        Self::Error(e.to_string())
    }
}

impl Error for NodeError {}
