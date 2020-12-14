//! Processes monitoring based on /proc filesystem

use std::io;

use thiserror::Error;

use crate::core::process_view::PID;

mod parsers;
mod rates;

pub mod process;

pub mod cpu_probe;
#[cfg(feature = "netio")]
pub mod net_io_probe;

#[derive(Error, Debug)]
pub enum ProcfsError {
    #[error("Invalid file content: '{0:?}'")]
    InvalidFileContent(String),
    #[error("Invalid file format: '{0:?}'")]
    InvalidFileFormat(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("PID is not known: '{0:?}'")]
    UnknownPID(PID),
    #[error("Not enough data to estimate rate")]
    NotEnoughData
}
