//! Platform-independant process monitoring

use std::{fmt, io};
use std::fmt::{Display, Formatter};

use thiserror::Error;

use crate::core::metrics::Metric;
use crate::core::process_view::PID;

pub mod process_view;
pub mod metrics;
pub mod values;

#[derive(Error, Debug)]
pub enum Error {
    // Error raised from trait implementors
    #[error("Error scanning process")]
    ScanProcessesError(#[source] Box<dyn std::error::Error>),
    #[error("Error reading process metadata")]
    ReadMetadataError(#[source] Box<dyn std::error::Error>),
    #[error("Probing error: '{0:?}'")]
    ProbingError(String, #[source] Box<dyn std::error::Error>),
    #[error("Invalid percent value: '{0:?}'")]
    InvalidPercentValue(f32),
    #[error("Unexpected label: '{0:?}'")]
    UnexpectedLabel(String),
    #[error("Invalid PID: '{0:?}'")]
    InvalidPID(PID),
    #[error("Invalid metric variant for label {0:?}: {1:?}")]
    InvalidMetricVariant(String, Metric),
    #[error("Metric label has already been defined: '{0:?}'")]
    DuplicateLabel(String),
    #[error(transparent)]
    IOError(#[from] io::Error),
}
