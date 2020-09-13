pub mod process_view;
pub mod metrics;
pub mod values;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    ScanProcessesError(String),
    ReadMetadataError(String),
    InvalidPercentValue(f32),
    InvalidLabel,
    IOError(String),
    ProbingError(String),
}

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::ScanProcessesError(s) => {
                format!("Error while scanning processes: {}", s)
            }
            Error::ReadMetadataError(s) => {
                format!("Error while reading processe data: {}", s)
            }
            Error::InvalidPercentValue(p) => format!("Invalid percent value: {}", *p),
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
            Error::InvalidLabel => format!("Invalid label"),
        }
    }
}
