//! Metric handling

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};
use std::fmt;
use std::iter::Skip;
use std::slice::Iter;
use std::time::Duration;

use log::error;

use crate::core::Error;
use crate::core::process_view::PID;
use crate::core::values::{Bitrate, Percent, Value};

/// A value probed from a process
#[derive(Debug, PartialEq)]
pub enum Metric {
    Percent(Percent),
    Bitrate(Bitrate),
}


type PercentType = <Percent as Value>::ValueType;
type BitrateType = <Bitrate as Value>::ValueType;

impl Metric {
    pub fn from_percent(pct: PercentType) -> Result<Metric, Error> {
        Percent::new(pct)
            .and_then(|p| Ok(Metric::Percent(p)))
    }

    pub fn from_bitrate(bitrate: BitrateType) -> Metric {
        Metric::Bitrate(Bitrate::new(bitrate))
    }

    pub fn unit(&self) -> String {
        match self {
            Metric::Percent(_) => self.unit(),
            Metric::Bitrate(_) => self.unit(),
        }
    }

    pub fn base_unit(&self) -> &'static str {
        match self {
            Metric::Percent(_) => "%",
            Metric::Bitrate(_) => "bps",
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            Metric::Percent(pct) => pct.value() as f64,
            Metric::Bitrate(br) => br.value() as f64,
        }
    }
}

impl PartialOrd for Metric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Metric::Percent(_), Metric::Bitrate(_)) => {
                panic!("Comparing incompatible metrics")
            }
            (Metric::Bitrate(_), Metric::Percent(_)) => {
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


impl Display for Metric {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Metric::Percent(pct) => pct.to_string(),
            Metric::Bitrate(br) => br.to_string(),
        };

        write!(f, "{}", repr)
    }
}

/// Types which can probe processes for a specific kind of [`Metric`](enum.Metric)
pub trait Probe {
    fn probe_name(&self) -> &'static str;
    
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<Metric, Error>;

    /// Returns a map associating a `Metric` instance to each PID
    ///
    /// This method might not return a metric value for all given processes, for instance if
    /// probing one process produces an error. TODO think this over
    /// TODO shouldn't probe rather be a trait with two methods: one init_iteration() and one probe
    ///
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    fn probe_processes(&mut self, pids: impl Iterator<Item=PID>) -> Result<HashMap<PID, Metric>, Error> {
        self.init_iteration()?;

        let metrics = pids.filter_map(|pid| {
            self.probe(pid)
                .map_err(|e| {
                    error!("Could not probe {} metric for pid {}: {}",
                           self.probe_name(), pid, e.to_string());
                    e
                })
                .ok()
                .map(|m| (pid, m))
        })
            .collect();

        Ok(metrics)
    }
}

/// Builder class to initialize an [`Archive`](struct.Archive.html)
pub struct ArchiveBuilder {
    archive: Archive
}

impl ArchiveBuilder {
    pub fn new() -> Self {
        let archive = Archive {
            metrics: HashMap::new(),
            resolution: Duration::from_secs(1),
        };
        ArchiveBuilder { archive }
    }

    pub fn resolution(mut self, resolution: Duration) -> Self {
        self.archive.resolution = resolution;
        self
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
    use std::time::Duration;

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

    #[test]
    fn test_set_archive_resolution() {
        let archive = ArchiveBuilder::new()
            .resolution(Duration::from_secs(123))
            .build();

        assert_eq!(archive.resolution, Duration::from_secs(123));
    }
}


/// Container for all collected metrics
pub struct Archive {
    metrics: HashMap<String, ProcessMetrics>,
    resolution: Duration,
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
    /// If `label` is invalid, returns a Error::InvalidLabel
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

    /// Get the latest metric entry for the given label and PID, or a default value if none exist
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe which produced the metric
    ///  * `pid`: The ID of the process for which to retrieve the latest metric
    ///
    /// If `label` is invalid, returns a Error::InvalidLabel
    ///
    pub fn last(&self, label: &str, pid: PID) -> Result<&Metric, Error> {
        self.metrics.get(label)
            .and_then(|pm| Some(pm.last(pid)))
            .ok_or(Error::InvalidLabel)
    }

    /// Get a textual representation of the unit of metrics pushed by the probe with the given label
    /// name
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the unit
    ///
    /// If `label` is invalid, returns a Error::InvalidLabel
    ///
    pub fn label_unit(&self, label: &str) -> Result<&'static str, Error> {
        self.metrics.get(label)
            .and_then(|pm| Some(pm.unit()))
            .ok_or(Error::InvalidLabel)
    }

    /// Returns an iterator over `Metric` for the given probe label and process ID.
    /// The iterator only contains metrics in the given span
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the unit
    ///  * `pid`: The ID of the process for which to retrieve the history
    ///  * `span`: Indicates from how long back to retrieve metrics. To see how many metrics can
    ///         be contained in the iterator based on this argument, see `expected_metrics()`
    ///
    /// If `label` is invalid, returns a Error::InvalidLabel
    ///
    pub fn history(&self, label: &str, pid: PID, span: Duration) -> Result<MetricIter, Error> {
        let metrics = self.metrics.get(label)
            .ok_or(Error::InvalidLabel)?;

        let metrics_count = metrics.count(pid);
        let collected_metrics = self.expected_metrics(span);
        let skipped_metrics = metrics_count.checked_sub(collected_metrics)
            .unwrap_or(0);

        Ok(MetricIter {
            iter: metrics.iter_process(pid)?
                .skip(skipped_metrics)
        })
    }

    /// Indicates how many metrics should be returned by history() with the given span, according
    /// to this archive's resolution.
    /// Note that The value returned by this function is not a guarantee. History() may return less.
    /// 
    /// # Arguments
    ///  * span: Indicates from how long ago should metrics be returned
    pub fn expected_metrics(&self, span: Duration) -> usize {
        (span.as_secs() / self.resolution.as_secs()) as usize
    }
}

/// An iterator over [`Metric`](enum.Metric.html)
pub struct MetricIter<'a> {
    iter: Skip<Iter<'a, Metric>>
}

impl<'a> Iterator for MetricIter<'a> {
    type Item = &'a Metric;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl DoubleEndedIterator for MetricIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

#[cfg(test)]
mod test_archive {
    use std::time::Duration;

    use rstest::*;

    use crate::core::Error;
    use crate::core::metrics::{Archive, ArchiveBuilder, Metric};

    #[fixture]
    fn archive() -> Archive {
        ArchiveBuilder::new()
            .resolution(Duration::from_secs(2))
            .new_metric("label".to_string(), Metric::from_bitrate(1))
            .unwrap()
            .build()
    }

    #[fixture]
    fn metrics() -> Vec<Metric> {
        (1..100).map(|i| {
            Metric::from_bitrate(i)
        }).collect::<Vec<Metric>>()
    }

    #[fixture]
    fn filled_archive(mut archive: Archive, metrics: Vec<Metric>) -> Archive {
        metrics.into_iter()
            .for_each(|m| {
                archive.push("label", 123, m);
            });

        archive
    }

    #[rstest]
    fn test_current_should_be_last_pushed(mut archive: Archive) {
        archive.push("label", 123, Metric::from_bitrate(123))
            .unwrap();
        archive.push("label", 123, Metric::from_bitrate(456))
            .unwrap();

        assert_eq!(archive.last("label", 123).unwrap(),
                   &Metric::from_bitrate(456));
    }

    #[rstest]
    fn test_current_should_be_default_when_no_push(archive: Archive) {
        assert_eq!(archive.last("label", 123).unwrap(),
                   &Metric::from_bitrate(1));
    }

    #[rstest]
    fn test_push_should_fail_when_first_variant_is_invalid(mut archive: Archive) {
        assert_eq!(archive.push("label", 123,
                                Metric::from_percent(45.1).unwrap()),
                   Err(Error::InvalidMetricVariant));
    }

    #[rstest]
    fn test_push_should_fail_when_additional_variant_is_invalid(mut archive: Archive) {
        archive.push("label", 123, Metric::from_bitrate(45))
            .unwrap();

        assert_eq!(archive.push("label", 123,
                                Metric::from_percent(50.).unwrap()),
                   Err(Error::InvalidMetricVariant));
    }

    #[rstest]
    fn test_push_should_fail_when_label_is_invalid(mut archive: Archive) {
        assert_eq!(archive.push("invalid-label", 123,
                                Metric::from_bitrate(123)),
                   Err(Error::InvalidLabel));
    }

    #[rstest]
    fn test_current_should_fail_when_label_is_invalid(archive: Archive) {
        assert_eq!(archive.last("invalid-label", 123),
                   Err(Error::InvalidLabel));
    }

    #[rstest]
    fn test_history_should_be_iterator_of_pushed_metrics(mut archive: Archive) {
        let mut expected_metrics = Vec::new();
        (1..10).for_each(|i| {
            archive.push("label", 123, Metric::from_bitrate(i));
            expected_metrics.push(Metric::from_bitrate(i));
        });

        let iter = archive.history("label", 123, Duration::from_secs(60))
            .unwrap();

        assert_eq!(iter.collect::<Vec<&Metric>>(),
                   expected_metrics.iter().collect::<Vec<&Metric>>());
    }

    #[rstest]
    fn test_history_with_same_span_and_resolution(filled_archive: Archive, metrics: Vec<Metric>) {
        let iter = filled_archive.history("label", 123,
                                          filled_archive.resolution)
            .unwrap();

        assert_eq!(iter.collect::<Vec<&Metric>>(),
                   vec![metrics.last().unwrap()])
    }

    #[rstest]
    fn test_history_with_double_span_than_resolution(filled_archive: Archive,
                                                     metrics: Vec<Metric>) {
        let iter = filled_archive.history("label", 123,
                                          filled_archive.resolution * 2)
            .unwrap();

        assert_eq!(iter.collect::<Vec<&Metric>>(),
                   metrics.iter().rev().take(2).rev().collect::<Vec<&Metric>>())
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

    fn unit(&self) -> &'static str {
        self.default.base_unit()
    }

    fn iter_process(&self, pid: PID) -> Result<Iter<Metric>, Error> {
        Ok(self.series.get(&pid)
            .ok_or(Error::InvalidPID)?
            .iter())
    }

    fn count(&self, pid: PID) -> usize {
        self.series.get(&pid)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}
