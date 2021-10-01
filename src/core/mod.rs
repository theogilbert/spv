//! Platform-independant process monitoring

use std::io;

use thiserror::Error;

use crate::core::process_view::Pid;

pub mod process_view;
pub mod metrics;
pub mod collection;
pub mod probe;
pub mod view;

#[derive(Error, Debug)]
pub enum Error {
    // Error raised from trait implementors
    #[error("Error scanning process: {0}")]
    ScanProcessesError(#[source] anyhow::Error),
    #[error("Error while probing metrics")]
    ProbingError(String, #[source] anyhow::Error),
    #[error("Unexpected label: '{0:?}'")]
    UnexpectedLabel(String),
    #[error("Invalid PID: '{0:?}'")]
    InvalidPID(Pid),
    #[error("Metric label has already been defined: '{0:?}'")]
    DuplicateLabel(String),
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error("Error accessing raw value {0:?} (cardinality: {1:?})")]
    RawMetricAccessError(usize, usize),
}
