use std::collections::HashSet;

use crate::probe::dispatch::Metrics;
use crate::process::PID;
use values::Value;

mod cpu;
mod dispatch;
mod procfs;
pub mod values;

/// Errors related to probing
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    IOError(String),
    ProbingError(String),
    MPSCError(String),
    ThreadKilledError,
}


impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
            Error::MPSCError(s) => format!("MSPC error: {}", s.clone()),
            Error::ThreadKilledError => "The thread has been killed".to_string(),
        }
    }
}

/// Contains a `Value` associated to a process
#[derive(Debug, PartialEq, Clone)]
pub struct ProcessMetric<T> where T: Value {
    pid: PID,
    value: T,
}

impl<T> ProcessMetric<T> where T: Value {
    pub fn new(pid: PID, value: T) -> Self {
        Self { pid, value }
    }
}

pub trait Probe {
    fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<Metrics, Error>;
}
