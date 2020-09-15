use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;

use crate::core::Error;
use crate::core::process_view::PID;
use crate::core::values::{Bitrate, Percent, Value};

#[derive(Debug, PartialEq)]
pub enum Metric {
    Percent(Percent),
    Bitrate(Bitrate),
}


type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

// #[cfg(test)]
impl Metric {
    pub fn from_percent(pct: PercentType) -> Result<Metric, Error> {
        Percent::new(pct)
            .and_then(|p| Ok(Metric::Percent(p)))
    }

    pub fn from_bitrate(bitrate: BitrateType) -> Metric {
        Metric::Bitrate(Bitrate::new(bitrate))
    }
}

impl PartialOrd for Metric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Metric::Percent(pct), Metric::Bitrate(br)) => {
                panic!("Comparing incompatible metrics")
            },
            (Metric::Bitrate(br), Metric::Percent(pct)) => {
                panic!("Comparing incompatible metrics")
            },
            (Metric::Percent(pct_self), Metric::Percent(pct_other)) => {
                pct_self.partial_cmp(pct_other)
            },
            (Metric::Bitrate(br_self), Metric::Bitrate(br_other)) => {
                br_self.partial_cmp(br_other)
            }
        }
    }
}

impl ToString for Metric {
    fn to_string(&self) -> String {
        match self {
            Metric::Percent(pct) => pct.to_string(),
            Metric::Bitrate(br) => br.to_string(),
        }
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
    metrics: HashMap<String, ProcessMetrics>
}

impl Archive {
    pub fn new(labels: Vec<String>) -> Self {
        let metrics = labels.into_iter()
            .map(|l| (l, ProcessMetrics::default()))
            .collect();

        Self { metrics }
    }

    pub fn push(&mut self, label: &str, pid: PID, metric: Metric) -> Result<(), Error> {
        match self.metrics.get_mut(label) {
            Some(pm) => {
                pm.push(pid, metric);
                Ok(())
            }
            None => Err(Error::InvalidLabel)
        }
    }

    pub fn current(&self, label: &str, pid: PID) -> Option<&Metric> {
        self.metrics.get(label)
            .and_then(|pm| pm.last(pid))
    }
}

#[cfg(test)]
mod test_archive {
    use crate::core::Error;
    use crate::core::metrics::{Archive, Metric};

    #[test]
    fn test_current_should_be_last_pushed() {
        let mut archive = Archive::new(vec!["label".to_string()]);

        archive.push("label", 123, Metric::from_bitrate(123))
            .unwrap();
        archive.push("label", 123, Metric::from_bitrate(456))
            .unwrap();

        assert_eq!(archive.current("label", 123),
                   Some(&Metric::from_bitrate(456)));
    }

    #[test]
    fn test_current_should_be_none_when_no_push() {
        let mut archive = Archive::new(vec!["label".to_string()]);

        assert_eq!(archive.current("label", 123), None);
    }

    #[test]
    fn test_push_should_fail_when_label_is_invalid() {
        let mut archive = Archive::new(vec!["label".to_string()]);

        let ret = archive.push("invalid-label", 123,
                               Metric::from_bitrate(123));

        assert_eq!(ret, Err(Error::InvalidLabel));
    }
}


struct ProcessMetrics {
    series: HashMap<PID, Vec<Metric>>
}

impl Default for ProcessMetrics {
    fn default() -> Self {
        Self { series: hashmap!() }
    }
}

impl ProcessMetrics {
    fn push(&mut self, pid: PID, metric: Metric) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new())
        };

        process_series.push(metric);
    }

    fn last(&self, pid: PID) -> Option<&Metric> {
        self.series.get(&pid)
            .and_then(|v| v.last())
    }
}