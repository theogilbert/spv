pub mod process;

// The different types of errors that can be returned within the spv application
mod spv {
    #[derive(Debug, Eq, PartialEq)]
    pub enum Error {
        ProcessScanningFailure(String),
        InvalidPidDirName,
        ProcessParsingError(String),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
