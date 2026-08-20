#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    convert::Infallible,
    io::{self},
    process::Termination as _T,
};

use clap::{self, Parser};
use exit_safely::Termination;
use try_v2::Try;

mod cli;
use cli::Tag;

fn main() -> Exit<()> {
    let tagger = Tag::try_parse()?;
    let tags = metaflac::Tag::read_from_path(tagger.filename)?;
    dbg!(tags);
    Exit::Ok(())
}

#[derive(Debug, Termination, Try, PartialEq, PartialOrd, Eq, Ord)]
#[FromResidual(Result<_, Self::Residual>)]
#[repr(u8)]
#[must_use]
pub enum Exit<T: _T> {
    Ok(T) = 0,
    Error(String) = 1,
    InvocationError(String) = 2,
    IO(String) = 3,
}

impl<T: _T> From<metaflac::Error> for Exit<T> {
    fn from(e: metaflac::Error) -> Self {
        Self::Error(e.to_string())
    }
}

impl<T: _T> From<clap::Error> for Exit<T> {
    fn from(e: clap::Error) -> Self {
        Self::InvocationError(e.to_string())
    }
}

impl<T: _T> From<io::Error> for Exit<T> {
    fn from(e: io::Error) -> Self {
        Self::IO(e.to_string())
    }
}

impl<T: _T> From<Infallible> for Exit<T> {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
