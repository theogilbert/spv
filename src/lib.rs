pub mod process;

// The different types of errors that can be returned within the spv application
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    ProcessScanningFailure(String),
    InvalidPidDirName,
}
