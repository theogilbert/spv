use std::collections::HashSet;

use crate::probe::thread::Metrics;
use crate::process::PID;
use crate::values::Value;

mod cpu;
mod thread;
mod procfs;

pub type CpuProbe = cpu::CpuProbe;

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

pub trait Probe {
    fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<Metrics, Error>;
}