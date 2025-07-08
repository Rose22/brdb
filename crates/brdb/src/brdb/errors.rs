use std::fmt::Display;

use thiserror::Error;

use crate::errors::BrFsError;

#[derive(Debug, Error)]
pub enum BrdbError {
    #[error("{0}: {1}")]
    Wrapped(String, Box<Self>),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("required table is missing: {0}")]
    MissingTable(&'static str),
    #[error(transparent)]
    Fs(#[from] BrFsError),
}

impl BrdbError {
    pub fn wrap(self, label: impl Display) -> Self {
        Self::Wrapped(label.to_string(), Box::new(self))
    }
}
