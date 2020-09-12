pub mod process_view;

pub enum Error {
    ScanProcessesError(String),
    ReadMetadataError(String),
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
        }
    }
}