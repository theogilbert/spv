//! Processes monitoring based on /proc filesystem

use std::io;

use thiserror::Error;

pub mod cpu_probe;
mod parsers;
pub mod process;

#[derive(Error, Debug)]
pub enum ProcfsError {
    #[error("Invalid file content: '{0:?}'")]
    InvalidFileContent(String),
    #[error("Invalid file format: '{0:?}'")]
    InvalidFileFormat(String),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
