use std::error::Error as StdError;
use std::time::Duration;

use log::error;
use netinfo::{InoutType, Netinfo, NetStatistics, Pid};
use thiserror::Error;

use crate::core::Error;
use crate::core::metrics::IOMetric;
use crate::core::probe::Probe;
use crate::procfs::rates::{ProcessesRates, PushMode};

const RATE_RETENTION: Duration = Duration::from_secs(5);


/// netinfo uses error-chain, which is an unmaintained library not compatible with anyhow
/// To be able to return netinfo error information, we convert it to this NetinfoError type
#[derive(Error, Debug)]
#[error("{msg}")]
struct NetinfoError {
    // As a netinfo::Error's source can also be a netinfo::Error, the soruces are recursively
    // converted to a NetInfoError
    #[source]
    source: Option<Box<NetinfoError>>,
    msg: String,
}

impl NetinfoError {
    /// Convert any trait object of std::error::Error to a `NetinfoError`
    /// We do not implement From<&dyn StdError> -> Self for this because it conflicts with From<T> -> T
    fn from_std_error(e: &dyn StdError) -> Self {
        let source = e.source().take()
            .map(|src| Box::new(Self::from_std_error(src)));

        NetinfoError { source, msg: e.to_string() }
    }

    fn from_string(msg: String) -> Self {
        NetinfoError { source: None, msg }
    }
}


pub struct NetIoProbe {
    net_info: Netinfo,
    input_processes_rates: ProcessesRates,
    output_processes_rates: ProcessesRates,
    net_stats: Option<NetStatistics>,
}

impl NetIoProbe {
    pub fn new() -> Result<Self, Error> {
        let net_ifs = Netinfo::list_net_interfaces()
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Error listing net interfaces".to_string(),
                                             e.into()))?;
        let mut net_info = Netinfo::new(&net_ifs)
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Could not initialize NetInfo".to_string(),
                                             e.into()))?;

        net_info.set_min_refresh_interval(Some(Duration::from_millis(100)))
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Could not configure net IO thread".to_string(),
                                             e.into()))?;
        net_info.start() // stop() is automatically called on drop()
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Could not start net IO thread".to_string(),
                                             e.into()))?;

        Ok(NetIoProbe {
            net_info,
            input_processes_rates: ProcessesRates::new(PushMode::Increment, RATE_RETENTION),
            output_processes_rates: ProcessesRates::new(PushMode::Increment, RATE_RETENTION),
            net_stats: None,
        })
    }
}

impl Probe<IOMetric> for NetIoProbe {
    fn name(&self) -> &'static str {
        "Net I/O"
    }

    fn init_iteration(&mut self) -> Result<(), Error> {
        let net_stats = self.net_info.get_net_statistics()
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Error getting net statistics".to_string(),
                                             e.into()))?;
        self.net_info.clear()
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| Error::ProbingError("Error clearing net io cache".to_string(),
                                             e.into()))?;

        self.net_stats = Some(net_stats);

        let errors = self.net_info.pop_thread_errors()
            .map_err(|e| NetinfoError::from_std_error(&e))
            .map_err(|e| {
                Error::ProbingError("Could not fetch net io thread errors".into(), e.into())
            })?;
        errors.iter()
            .for_each(|e| {
                error!("Error while parsing net packets: {:?}", e);
            });

        Ok(())
    }

    fn probe(&mut self, pid: u32) -> Result<IOMetric, Error> {
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
                                                 e.into()))?;
            let output_rate = self.output_processes_rates.rate(pid)
                .map_err(|e| Error::ProbingError("Error calculating output rate".into(),
                                                 e.into()))?;

            Ok(IOMetric::new(input_rate as usize, output_rate as usize))
        } else {
            let error_msg = "Cannot probe net I/O: Net stats are not set.".to_string();

            Err(Error::ProbingError("Error listing net interfaces".to_string(),
                                    NetinfoError::from_string(error_msg).into()))
        }
    }
}
