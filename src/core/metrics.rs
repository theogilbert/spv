use crate::core::Error;
use crate::core::values::{Bitrate, Percent, Value};
use std::collections::{HashSet, HashMap};
use crate::core::process_view::PID;

#[derive(Debug, PartialEq)]
pub enum Metric {
    Percent(Percent),
    Bitrate(Bitrate),
}


type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

#[cfg(test)]
impl Metric {
    pub fn from_percent(pct: PercentType) -> Result<Metric, Error> {
        Percent::new(pct)
            .and_then(|p| Ok(Metric::Percent(p)))
    }

    pub fn from_bitrate(bitrate: BitrateType) -> Metric {
        Metric::Bitrate(Bitrate::new(bitrate))
    }
}

/// A trait for the ability to measure metrics of processes given their `PIDs`
pub trait Probe {
    /// Returns a map associating a `Metric` instance to each PID
    ///
    /// This method might not return a metric value for all given processes, for instance if
    /// probing one process produces an error. TODO think this over
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    fn probe_processes(&mut self, pids: &HashSet<PID>) -> Result<HashMap<PID, Metric>, Error>;
}


pub struct Archive {
    metrics: HashMap<String, HashMap<PID, Vec<Metric>>>
}