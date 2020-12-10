use crate::core::Error;
use crate::core::metrics::{Metric, Probe};

// TODO can press key to configure to show only input/only output
pub struct NetIoProbe {}

impl Default for NetIoProbe {
    fn default() -> Self {
        NetIoProbe {}
    }
}

impl Probe for NetIoProbe {
    fn name(&self) -> &'static str {
        "Net I/O"
    }

    fn default_metric(&self) -> Metric {
        Metric::IO(0, 0)
    }

    fn probe(&mut self, pid: u32) -> Result<Metric, Error> {
        Ok(self.default_metric())
    }
}

#[cfg(test)]
mod test_net_io_probe {
    // TODO
}