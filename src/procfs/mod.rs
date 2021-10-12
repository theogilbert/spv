//! Processes monitoring based on /proc filesystem

use std::io;

use thiserror::Error;

use crate::core::process::Pid;

mod parsers;

pub mod process;

pub mod cpu_probe;
// pub mod mem_probe;

#[cfg(feature = "netio")]
pub mod net_io_probe;
#[cfg(feature = "netio")]
mod rates;

#[derive(Error, Debug)]
pub enum ProcfsError {
    #[error("Invalid file content: '{0:?}'")]
    InvalidFileContent(String),
    #[error("Invalid file format: '{0:?}'")]
    InvalidFileFormat(String),
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error("PID is not known: '{0:?}'")]
    UnknownPID(Pid),
    #[error("Not enough data to estimate rate")]
    NotEnoughData,
}
