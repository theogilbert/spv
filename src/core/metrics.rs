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
            }
            (Metric::Bitrate(br), Metric::Percent(pct)) => {
                panic!("Comparing incompatible metrics")
            }
            (Metric::Percent(pct_self), Metric::Percent(pct_other)) => {
                pct_self.partial_cmp(pct_other)
            }
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


pub struct ArchiveBuilder {
    archive: Archive
}

impl ArchiveBuilder {
    pub fn new() -> Self {
        let archive = Archive { metrics: HashMap::new() };
        ArchiveBuilder { archive }
    }

    pub fn new_metric(mut self, label: String, default: Metric) -> Result<Self, Error> {
        match self.archive.metrics.insert(label, ProcessMetrics::new(default)) {
            Some(_) => Err(Error::DuplicateLabel),
            None => Ok(self)
        }
    }

    pub fn build(self) -> Archive {
        self.archive
    }
}


#[cfg(test)]
mod test_archive_builder {
    use crate::core::Error;
    use crate::core::metrics::ArchiveBuilder;
    use crate::core::metrics::Metric;

    #[test]
    fn test_should_return_error_on_duplicate_label() {
        let err_ret = ArchiveBuilder::new()
            .new_metric("label".to_string(), Metric::from_bitrate(123))
            .unwrap()
            .new_metric("label".to_string(), Metric::from_bitrate(123));

        match err_ret {
            Ok(_) => panic!("Should have failed"),
            Err(Error::DuplicateLabel) => (),
            Err(_) => panic!("Should have been duplicate error"),
        };
    }
}


/// Represents all collected metrics for all processes
pub struct Archive {
    metrics: HashMap<String, ProcessMetrics>,
}

impl Archive {
    /// Pushes a new `Metric` to the archive
    /// If the label is invalid, `Error::InvalidLabel` will be returned
    /// If the metric variant is incompatible with the label, `Error::InvalidMetricVariant` will be
    ///  returned
    ///
    /// # Arguments
    ///  * `label` The name of the label of the metric
    ///  * `pid` The ID of the process from which comes the `Metric`
    ///  * `metric` The new metric to associate to the given process and label
    ///                 Only one variant of `Metric` is allowed per label
    ///
    pub fn push(&mut self, label: &str, pid: PID, metric: Metric) -> Result<(), Error> {
        let pm = match self.metrics.get_mut(label) {
            Some(pm) => Ok(pm),
            None => Err(Error::InvalidLabel)
        }?;

        Ok(match (pm.last(pid), metric) {
            (&Metric::Percent(_), Metric::Percent(pct)) => pm.push(pid, Metric::Percent(pct)),
            (&Metric::Bitrate(_), Metric::Bitrate(br)) => pm.push(pid, Metric::Bitrate(br)),
            (_, _) => Err(Error::InvalidMetricVariant)?
        })
    }

    pub fn current(&self, label: &str, pid: PID) -> Result<&Metric, Error> {
        self.metrics.get(label)
            .ok_or(Error::InvalidLabel)
            .and_then(|pm| Ok(pm.last(pid)))
    }
}

#[cfg(test)]
mod test_archive {
    use crate::core::Error;
    use crate::core::metrics::{Archive, ArchiveBuilder, Metric};

    #[test]
    fn test_current_should_be_last_pushed() {
        let mut archive = ArchiveBuilder::new()
            .new_metric("label".to_string(), Metric::from_bitrate(1))
            .unwrap()
            .build();

        archive.push("label", 123, Metric::from_bitrate(123))
            .unwrap();
        archive.push("label", 123, Metric::from_bitrate(456))
            .unwrap();

        assert_eq!(archive.current("label", 123).unwrap(),
                   &Metric::from_bitrate(456));
    }

    #[test]
    fn test_current_should_be_default_when_no_push() {
        let mut archive = ArchiveBuilder::new()
            .new_metric("label".to_string(), Metric::from_percent(45.1).unwrap())
            .unwrap()
            .build();

        assert_eq!(archive.current("label", 123).unwrap(),
                   &Metric::from_percent(45.1).unwrap());
    }

    #[test]
    fn test_push_should_fail_when_first_variant_is_invalid() {
        let mut archive = ArchiveBuilder::new()
            .new_metric("label".to_string(), Metric::from_percent(45.1).unwrap())
            .unwrap()
            .build();

        assert_eq!(archive.push("label", 123,
                                Metric::from_bitrate(123)),
                   Err(Error::InvalidMetricVariant));
    }

    #[test]
    fn test_push_should_fail_when_additional_variant_is_invalid() {
        let mut archive = ArchiveBuilder::new()
            .new_metric("label".to_string(), Metric::from_percent(45.1).unwrap())
            .unwrap()
            .build();

        archive.push("label", 123, Metric::from_percent(45.1).unwrap())
            .unwrap();

        assert_eq!(archive.push("label", 123,
                                Metric::from_bitrate(123)),
                   Err(Error::InvalidMetricVariant));
    }

    #[test]
    fn test_push_should_fail_when_label_is_invalid() {
        let mut archive = ArchiveBuilder::new()
            .build();

        assert_eq!(archive.push("invalid-label", 123,
                                Metric::from_bitrate(123)),
                   Err(Error::InvalidLabel));
    }

    #[test]
    fn test_current_should_fail_when_label_is_invalid() {
        let mut archive = ArchiveBuilder::new()
            .build();

        assert_eq!(archive.current("invalid-label", 123),
                   Err(Error::InvalidLabel));
    }
}

struct ProcessMetrics {
    default: Metric,
    series: HashMap<PID, Vec<Metric>>,
}

impl ProcessMetrics {
    fn new(default: Metric) -> Self {
        Self { default, series: hashmap!() }
    }

    fn push(&mut self, pid: PID, metric: Metric) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new())
        };

        process_series.push(metric);
    }

    fn last(&self, pid: PID) -> &Metric {
        self.series.get(&pid)
            .and_then(|v| v.last())
            .unwrap_or(&self.default)
    }
}
