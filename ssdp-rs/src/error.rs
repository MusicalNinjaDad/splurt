//! Top level error handling.

use std::io;

use derive_more::Display;

use crate::message::{ErrorKind, ParseError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Display)]
pub enum Error {
    ParseError(ParseError),
    IOError(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IOError(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self::ParseError(err)
    }
}

impl From<ErrorKind> for Error {
    fn from(err: ErrorKind) -> Self {
        Self::ParseError(err.into())
    }
}
