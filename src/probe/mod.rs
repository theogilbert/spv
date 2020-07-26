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

/// Contains a `Value` associated to a process
#[derive(Debug, PartialEq, Clone)]
pub struct ProcessMetric<T> where T: Value {
    pid: PID,
    value: T,
}

pub trait Probe {
    type ValueType: Value;

    /// Allow the initialization of the probe for the current iteration
    /// This method should be called before calling the probe() method for each pid
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Probe a specific metric value for a given process
    /// # Arguments
    ///  * `pid`: The ID of the process to probe
    ///
    fn probe(&mut self, pid: PID) -> Result<ProcessMetric<Self::ValueType>, Error>;
}