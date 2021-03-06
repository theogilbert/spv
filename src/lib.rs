use std::sync::mpsc;

use thiserror::Error;

#[cfg(test)]
#[macro_use]
mod macros;

pub mod core;
mod ctrl;
pub mod procfs;
pub mod spv;
pub mod triggers;
mod ui;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    MpscError(#[from] mpsc::RecvError),
    #[error(transparent)]
    UiError(#[from] ui::Error),
    #[error(transparent)]
    CoreError(#[from] core::Error),
}
