use std::collections::HashSet;

use crate::probe::thread::ProbedFrame;
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
}


impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::IOError(s) => format!("IO error: {}", s.clone()),
            Error::ProbingError(s) => format!("Probing error: {}", s.clone()),
            Error::MPSCError(s) => format!("MSPC error: {}", s.clone())
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
    fn probe_frame(&mut self, pids: &HashSet<PID>) -> Result<ProbedFrame, Error>;
}