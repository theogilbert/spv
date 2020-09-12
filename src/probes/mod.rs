use std::collections::HashSet;

pub use crate::probes::dispatch::{MetricSet, Metrics, ProbeDispatcher};
use crate::core::process_view::PID;

mod cpu;
mod dispatch;
mod procfs;
pub mod values;
pub mod process;

/// Errors related to probing
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    InvalidPercentValue(f32),
    IOError(String),
    ProbingError(String),
}


impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::InvalidPercentValue(p) => format!("Invalid percent value: {}", *p),
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
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
