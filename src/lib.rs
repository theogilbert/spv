pub mod process;

// The different types of errors that can be returned within the spv application
mod spv {
    #[derive(Debug, Eq, PartialEq, Clone)]
    pub enum Error {
        ProcessScanningFailure(String),
        ProcessParsingError(String),
        InvalidPID,
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
