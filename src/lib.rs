use std::fmt::{Display, Formatter};
use std::fmt;

#[cfg(test)]
#[macro_use]
mod macros;

mod ui;
pub mod procfs;
pub mod core;
pub mod triggers;
pub mod spv;

// TODO rework the error handling part of the whole application
#[derive(Debug)]
pub enum Error {
    MpscError(String),
    UiError(String),
    CoreError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Error::MpscError(s) => format!("MSPC error: {}", s),
            Error::UiError(s) => format!("Rendering error: {}", s),
            Error::CoreError(s) => format!("Core error: {}", s),
        };

        write!(f, "{}", repr)
    }
}
