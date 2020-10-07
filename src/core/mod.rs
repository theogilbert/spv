//! Platform-independant process monitoring

use std::fmt::{Display, Formatter};
use std::fmt;

pub mod process_view;
pub mod metrics;
pub mod values;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    ScanProcessesError(String),
    ReadMetadataError(String),
    InvalidPercentValue(f32),
    InvalidLabel,
    InvalidPID,
    InvalidMetricVariant,
    DuplicateLabel,
    IOError(String),
    ProbingError(String),
}

impl Display for Error {

    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Error::ScanProcessesError(s) => {
                format!("Error while scanning processes: {}", s)
            }
            Error::ReadMetadataError(s) => {
                format!("Error while reading processe data: {}", s)
            }
            Error::InvalidPercentValue(p) => format!("Invalid percent value: {}", *p),
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
            Error::InvalidLabel => "Invalid label".to_string(),
            Error::DuplicateLabel => "Duplicate label".to_string(),
            Error::InvalidMetricVariant => "Invalid metric variant".to_string(),
            Error::InvalidPID => "Invalid PID".to_string()
        };

        write!(f, "{}", repr)
    }
}
