pub mod process_view;

pub enum Error {
    ScanProcessesError(String),
    ReadMetadataError(String),
}