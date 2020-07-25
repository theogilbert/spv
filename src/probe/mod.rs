use crate::values::{BitrateValue, PercentValue};
use crate::process::PID;

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

// Probe metrics stuff

#[derive(Debug, PartialEq, Clone)]
pub enum Metric {
    IoRead(BitrateValue),
    IoWrite(BitrateValue),
    NetDesc(BitrateValue),
    NetAsc(BitrateValue),
    CpuUsage(PercentValue),
    MemUsage(PercentValue),
}

/// Contains a `Value` associated to a process
#[derive(Debug, PartialEq, Clone)]
pub struct ProcessMetric {
    pid: PID,
    value: Metric,
}

pub trait Probe {
    /// Allow the initialization of the probe for the current iteration
    /// This method should be called before calling the probe() method for each pid
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Probe a specific metric value for a given process
    /// # Arguments
    ///  * `pid`: The ID of the process to probe
    ///
    fn probe(&mut self, pid: PID) -> Result<ProcessMetric, Error>;
}