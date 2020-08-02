use std::collections::HashSet;

use crate::probe::dispatch::Metrics;
use crate::process::PID;

mod cpu;
mod dispatch;
mod procfs;
pub mod values;

/// Errors related to probing
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    InvalidPercentValue(f32),
    IOError(String),
    ProbingError(String),
    MPSCError(String),
    ThreadKilledError,
}


impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::InvalidPercentValue(p) => format!("Invalid percent value: {}", *p),
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
            Error::MPSCError(s) => format!("MSPC error: {}", s.clone()),
            Error::ThreadKilledError => "The thread has been killed".to_string(),
        }
    }
}

pub trait Probe {
    fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<Metrics, Error>;
}
