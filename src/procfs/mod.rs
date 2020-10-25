//! Processes monitoring based on /proc filesystem

use std::fmt::{Display, Formatter};
use std::io;

use thiserror::Error;

use crate::fmt;

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
