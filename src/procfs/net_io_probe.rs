use std::time::Duration;

use log::{error, info};
use netinfo::{InoutType, Netinfo, NetStatistics, Pid};

use crate::core::Error;
use crate::core::metrics::{Metric, Probe};
use crate::procfs::rates::ProcessesRates;

const RATE_RETENTION: Duration = Duration::from_secs(5);

// TODO can press key to configure to show only input/only output
pub struct NetIoProbe {
    net_info: Netinfo,
    input_processes_rates: ProcessesRates,
    output_processes_rates: ProcessesRates,
    net_stats: Option<NetStatistics>,
}

impl NetIoProbe {
    pub fn new() -> Result<Self, Error> {
        let net_ifs = Netinfo::list_net_interfaces()
            .map_err(|e| Error::ProbingError(format!("Error listing net interfaces"),
                                             Box::new(e)))?;
        let mut net_info = Netinfo::new(&net_ifs)
            .map_err(|e| Error::ProbingError(format!("Could not initialize NetInfo"),
                                             Box::new(e)))?;

        net_info.set_min_refresh_interval(Some(Duration::from_millis(20)))
            .map_err(|e| Error::ProbingError("Could not configure net IO thread".into(),
                                             Box::new(e)))?;
        net_info.start() // stop() is automatically called on drop()
            .map_err(|e| Error::ProbingError("Could not start net IO thread".into(),
                                             Box::new(e)))?;

        Ok(NetIoProbe {
            net_info,
            input_processes_rates: ProcessesRates::new(RATE_RETENTION),
            output_processes_rates: ProcessesRates::new(RATE_RETENTION),
            net_stats: None,
        })
    }
}

impl Probe for NetIoProbe {
    fn name(&self) -> &'static str {
        "Net I/O"
    }

    fn default_metric(&self) -> Metric {
        Metric::IOBps(0, 0)
    }

    fn init_iteration(&mut self) -> Result<(), Error> {
        let net_stats = self.net_info.get_net_statistics()
            .map_err(|e| Error::ProbingError(format!("Error getting net statistics"),
                                             Box::new(e)))?;

        self.net_stats = Some(net_stats);

        let errors = self.net_info.pop_thread_errors()
            .map_err(|e| {
                Error::ProbingError("Could not fetch net io thread errors".into(), Box::new(e))
            })?;
        errors.iter()
            .for_each(|e| {
                error!("Error while parsing net packets: {:?}", e);
            });

        Ok(())
    }

    fn probe(&mut self, pid: u32) -> Result<Metric, Error> {
        if let Some(net_stats) = &self.net_stats {
            let input = net_stats.get_bytes_by_attr(Some(pid as Pid),
                                                    Some(InoutType::Incoming),
                                                    None);
            let output = net_stats.get_bytes_by_attr(Some(pid as Pid),
                                                     Some(InoutType::Outgoing),
                                                     None);

            self.input_processes_rates.push(pid, input as usize);
            self.output_processes_rates.push(pid, output as usize);

            let input_rate = self.input_processes_rates.rate(pid)
                .map_err(|e| Error::ProbingError("Error calculating input rate".into(),
                                                 Box::new(e)))?;
            let output_rate = self.output_processes_rates.rate(pid)
                .map_err(|e| Error::ProbingError("Error calculating output rate".into(),
                                                 Box::new(e)))?;


            if input > 0 || output > 0 {
                info!("PID: {}. Input: {} / Output: {}. Rates: Input: {} / Output: {}", pid, input, output, input_rate, output_rate);
            }

            Ok(Metric::IOBps(input_rate as usize, output_rate as usize))
        } else {
            error!("Cannot probe net I/O: Net stats are not set.");
            Ok(self.default_metric())
        }
    }
}

#[cfg(test)]
mod test_net_io_probe {
    // TODO
}