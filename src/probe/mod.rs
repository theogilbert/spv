use std::collections::HashSet;

pub use crate::probe::dispatch::{Frame, Metrics, ProbeDispatcher};
use crate::probe::procfs::{PidStat, ProcessDataReader, Stat, SystemDataReader};
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

/// A trait for the ability to measure metrics of processes given their `PIDs`
pub trait Probe {
    /// Returns a `Metrics` instance containing the measured metrics for the given `PIDs`
    ///
    /// This method might not return a metric value for all given processes, for instance if
    /// probing one process produces an error.
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<Metrics, Error>;
}