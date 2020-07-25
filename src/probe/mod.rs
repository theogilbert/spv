pub mod cpu;
mod procfs;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use crate::probe::procfs::{PidStat, ProcfsReader, Stat};
use crate::process::PID;
use crate::values::{BitrateValue, PercentValue};


/// Errors related to probing
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    IOError(String),
    ProbingError(String),
    MPSCError(String),
}

// Probe metrics stuff

pub enum Metric {
    IoRead(BitrateValue),
    IoWrite(BitrateValue),
    NetDesc(BitrateValue),
    NetAsc(BitrateValue),
    CpuUsage(PercentValue),
    MemUsage(PercentValue),
}

/// Contains a `Value` associated to a process
pub struct ProcessMetric {
    pid: PID,
    value: Metric,
}

/// Contains a list of `ProcessMetric`, one for each probed process
pub struct ProbedFrame {
    metrics: Vec<ProcessMetric>,
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