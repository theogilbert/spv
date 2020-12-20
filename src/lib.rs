use std::sync::mpsc;

use thiserror::Error;

#[cfg(test)]
#[macro_use]
mod macros;

mod ui;
pub mod procfs;
pub mod core;
pub mod triggers;
pub mod spv;


#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    MpscError(#[from] mpsc::RecvError),
    #[error(transparent)]
    UiError(#[from] ui::Error),
    #[error(transparent)]
    CoreError(#[from] core::Error),
}
