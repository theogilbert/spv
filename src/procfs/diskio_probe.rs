//! CPU Usage probing

use crate::core::metrics::IOMetric;
use crate::core::probe::Probe;
use crate::core::process::Pid;
use crate::core::Error;

/// Probe implementation to measure the CPU usage (in percent) of processes
pub struct DiskIOProbe {}

impl DiskIOProbe {
    pub fn default() -> Self {
        DiskIOProbe {}
    }
}

impl Probe<IOMetric> for DiskIOProbe {
    fn name(&self) -> &'static str {
        "Disk I/O"
    }

    fn probe(&mut self, pid: Pid) -> Result<IOMetric, Error> {
        Ok(IOMetric::default())
    }
}
