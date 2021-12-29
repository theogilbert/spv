//! Platform-independant process monitoring

use std::io;

use thiserror::Error;

use crate::core::process::Pid;

pub mod collection;
pub mod metrics;
pub mod ordering;
pub mod probe;
pub mod process;
pub mod time;
pub mod view;

#[derive(Error, Debug)]
pub enum Error {
    // Error raised from trait implementors
    #[error("Error scanning process: {0}")]
    ScanProcessesError(#[source] anyhow::Error),
    #[error("{0}: {1:?}")]
    ProbingError(String, #[source] anyhow::Error),
    #[error("Invalid PID: '{0:?}'")]
    InvalidPID(Pid),
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error("Error accessing raw value {0:?} (cardinality: {1:?})")]
    RawMetricAccessError(usize, usize),
}
