//! Metric handling

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fmt;
use std::iter::Skip;
use std::ops::Sub;
use std::slice::Iter;
use std::time::Duration;

use log::warn;

use crate::core::Error;
use crate::core::process_view::PID;

/// A value probed from a process
#[derive(Debug, PartialEq, Clone)]
pub enum Metric {
    Percent(f64),
    Bitrate(usize),
    /// Input / Output rates, in bytes per seconds
    IO(usize, usize),
}


impl Metric {
    /// Returns the base unit of the metric
    pub fn unit(&self) -> &'static str {
        match self {
            Metric::Percent(_) => "%",
            Metric::Bitrate(_) => "B/s",
            Metric::IO(_, _) => "B/s"
        }
    }

    /// Returns a raw value from the metric, as f64
    ///
    /// # Arguments
    ///   * `index`: The value to retrieve from the Metric. Must not be greater than or equal to
    ///         `Metric::cardinality()`
    ///
    /// If the metric is the variant `Metric::IO`, `index=0` will return the input rate, whereas
    ///   `index=1` will return the output rate.
    pub fn raw_as_f64(&self, index: usize) -> Result<f64, Error> {
        if index >= self.cardinality() {
            Err(Error::RawMetricAccessError(index, self.cardinality()))
        } else {
            Ok(match self {
                Metric::Percent(pct) => *pct,
                Metric::Bitrate(br) => *br as f64,
                Metric::IO(input, output) => {
                    match index {
                        0 => *input as f64,
                        1 => *output as f64,
                        _ => panic!("Invalid raw value index")
                    }
                }
            })
        }
    }

    /// Indicates how many value the metric is composed of
    pub fn cardinality(&self) -> usize {
        match self {
            Metric::Percent(_) => 1,
            Metric::Bitrate(_) => 1,
            Metric::IO(_, _) => 2,
        }
    }

    /// Returns a more readable version of `bytes_val`
    /// `formatted_bytes(1294221)` -> 1.2M
    fn formatted_bytes(bytes_val: usize) -> String {
        if bytes_val == 0 {
            return "0".to_string()
        }

        const METRIC_PREFIXES: [&'static str; 4] = ["", "k", "M", "G"];

        let log = (bytes_val as f64).log(1024.)
            .sub(1.).max(0.).ceil() as usize;

        let prefix_index = log.min(METRIC_PREFIXES.len() - 1);

        let simplified = bytes_val as f64 / (1024_usize.pow(log as u32) as f64);

        format!("{:.1}{}", simplified, METRIC_PREFIXES[prefix_index])
    }

    /// An alternative to to_string(), but more concise to fit in places where space is important
    pub fn concise_repr(&self) -> String {
        match self {
            Metric::Percent(pct) => format!("{:.1}", pct),
            Metric::Bitrate(br) => {
                Self::formatted_bytes(*br)
            }
            Metric::IO(input, output) => {
                let reported_metric = input.max(output);
                Self::formatted_bytes(*reported_metric)
            }
        }
    }
}

impl PartialOrd for Metric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Metric::Percent(pct_self), Metric::Percent(pct_other)) => {
                pct_self.partial_cmp(pct_other)
            }
            (Metric::Bitrate(br_self), Metric::Bitrate(br_other)) => {
                br_self.partial_cmp(br_other)
            }
            (Metric::IO(input_1, output_1), Metric::IO(input_2, output_2)) => {
                (input_1 + output_1).partial_cmp(&(input_2 + output_2))
            }
            (_, _) => panic!("Comparing incompatible metrics"),
        }
    }
}


impl Display for Metric {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Metric::Percent(pct) => write!(f, "{:.1}", pct),
            Metric::Bitrate(br) => {
                write!(f, "{:}", Self::formatted_bytes(*br))
            }
            Metric::IO(input, output) => {
                write!(f, "{:}/{:}",
                       Self::formatted_bytes(*input),
                       Self::formatted_bytes(*output))
            }
        }
    }
}

/// Types which can probe processes for a specific kind of [`Metric`](enum.Metric)
pub trait Probe {
    fn name(&self) -> &'static str;

    fn default_metric(&self) -> Metric;

    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<Metric, Error>;

    /// Returns a map associating a `Metric` instance to each PID
    ///
    /// If a process is not probed correctly, a default value for the given probe is returned
    /// and a WARNING level log is produced
    ///
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    ///
    fn probe_processes(&mut self, pids: &[PID]) -> Result<HashMap<PID, Metric>, Error> {
        self.init_iteration()?;

        let metrics = pids.iter()
            .map(|pid| {
                let metric = self.probe(*pid)
                    .unwrap_or_else(|e| {
                        warn!("Could not probe {} metric for pid {}: {}",
                              self.name(), pid, e.to_string());
                        self.default_metric()
                    });

                (*pid, metric)
            })
            .collect();

        Ok(metrics)
    }
}

#[cfg(test)]
mod test_probe_trait {
    use std::collections::HashMap;

    use rstest::*;

    use crate::core::Error;
    use crate::core::metrics::{Metric, Probe};
    use crate::core::process_view::PID;

    struct FakeProbe {
        probe_responses: HashMap<PID, Metric>
    }

    impl Probe for FakeProbe {
        fn name(&self) -> &'static str { "fake-probe" }

        fn default_metric(&self) -> Metric { Metric::Bitrate(0) }

        fn probe(&mut self, pid: u32) -> Result<Metric, Error> {
            self.probe_responses.remove(&pid)
                .ok_or(Error::InvalidPID(pid))
        }
    }

    #[rstest]
    fn test_should_return_all_probed_values() {
        let mut probe = FakeProbe {
            probe_responses: hashmap!(1 => Metric::Bitrate(10), 2 => Metric::Bitrate(20))
        };

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&Metric::Bitrate(10)));
        assert_eq!(results.get(&2), Some(&Metric::Bitrate(20)));
    }

    #[rstest]
    fn test_should_return_default_value_if_probing_fails() {
        let mut probe = FakeProbe { probe_responses: hashmap!(1 => Metric::Bitrate(10)) };

        let results = probe.probe_processes(&[1, 2]).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results.get(&1), Some(&Metric::Bitrate(10)));
        assert_eq!(results.get(&2), Some(&probe.default_metric()));
    }
}

/// Builder class to initialize an [`Archive`](struct.Archive.html)
pub struct ArchiveBuilder {
    archive: Archive
}

impl Default for ArchiveBuilder {
    fn default() -> Self {
        Self::new()
    }
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
        match self.archive.metrics.entry(label.to_string()) {
            Entry::Occupied(_) => Err(Error::DuplicateLabel(label)),
            Entry::Vacant(entry) => {
                entry.insert(ProcessMetrics::new(default));
                Ok(self)
            }
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
            .new_metric("label".to_string(), Metric::Bitrate(123))
            .unwrap()
            .new_metric("label".to_string(), Metric::Bitrate(123));

        match err_ret {
            Ok(_) => panic!("Should have failed"),
            Err(Error::DuplicateLabel(_)) => (),
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
    /// If the label is invalid, `Error::UnexpectedLabel` will be returned
    /// If the metric variant is incompatible with the label, `Error::InvalidMetricVariant` will be
    ///  returned
    ///
    /// # Arguments
    ///  * `label` The name of the label of the metric
    ///  * `pid` The ID of the process from which comes the `Metric`
    ///  * `metric` The new metric to associate to the given process and label
    ///                 Only one variant of `Metric` is allowed per label
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn push(&mut self, label: &str, pid: PID, metric: Metric) -> Result<(), Error> {
        let pm = match self.metrics.get_mut(label) {
            Some(pm) => Ok(pm),
            None => Err(Error::UnexpectedLabel(label.to_string()))
        }?;

        match (pm.last(pid), metric) {
            (&Metric::Percent(_), Metric::Percent(pct)) => pm.push(pid, Metric::Percent(pct)),
            (&Metric::Bitrate(_), Metric::Bitrate(br)) => pm.push(pid, Metric::Bitrate(br)),
            (&Metric::IO(_, _), Metric::IO(input, output)) => pm.push(pid, Metric::IO(input, output)),
            (_, m) => return Err(Error::InvalidMetricVariant(label.to_string(), m))
        };

        Ok(())
    }

    /// Get the latest metric entry for the given label and PID, or a default value if none exist
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe which produced the metric
    ///  * `pid`: The ID of the process for which to retrieve the latest metric
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn last(&self, label: &str, pid: PID) -> Result<&Metric, Error> {
        self.metrics.get(label)
            .map(|pm| pm.last(pid))
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))
    }

    /// Get a textual representation of the unit of metrics pushed by the probe with the given label
    /// name
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the unit
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn label_unit(&self, label: &str) -> Result<&'static str, Error> {
        self.metrics.get(label)
            .map(|pm| pm.unit())
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))
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
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn history(&self, label: &str, pid: PID, span: Duration) -> Result<MetricIter, Error> {
        let metrics = self.metrics.get(label)
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))?;

        let metrics_count = metrics.count(pid);
        let collected_metrics = self.expected_metrics(span);
        let skipped_metrics = metrics_count.saturating_sub(collected_metrics);

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

    /// Get the default `Metric` associated to the probe with the given label
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the default `Metric`
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn default_metric(&self, label: &str) -> Result<Metric, Error> {
        self.metrics.get(label)
            .map(|pm| pm.default())
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))
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
            .new_metric("label".to_string(), Metric::Bitrate(1))
            .unwrap()
            .build()
    }

    #[fixture]
    fn metrics() -> Vec<Metric> {
        (1..100).map(|i| {
            Metric::Bitrate(i)
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
        archive.push("label", 123, Metric::Bitrate(123))
            .unwrap();
        archive.push("label", 123, Metric::Bitrate(456))
            .unwrap();

        assert_eq!(archive.last("label", 123).unwrap(),
                   &Metric::Bitrate(456));
    }

    #[rstest]
    fn test_current_should_be_default_when_no_push(archive: Archive) {
        assert_eq!(archive.last("label", 123).unwrap(),
                   &Metric::Bitrate(1));
    }

    #[rstest]
    fn test_push_should_fail_when_first_variant_is_invalid(mut archive: Archive) {
        let label = "label".to_string();

        assert!(matches!(archive.push(&label, 123, Metric::Percent(45.1)),
                         Err(Error::InvalidMetricVariant(label, _))));
    }

    #[rstest]
    fn test_push_should_fail_when_additional_variant_is_invalid(mut archive: Archive) {
        let label = "label".to_string();
        archive.push(&label, 123, Metric::Bitrate(45))
            .unwrap();

        assert!(matches!(archive.push(&label, 123, Metric::Percent(50.)),
                         Err(Error::InvalidMetricVariant(label, _))));
    }

    #[rstest]
    fn test_push_should_fail_when_label_is_invalid(mut archive: Archive) {
        let label = "invalid-label".to_string();
        assert!(matches!(archive.push(&label, 123, Metric::Bitrate(123)),
                         Err(Error::UnexpectedLabel(label))));
    }

    #[rstest]
    fn test_current_should_fail_when_label_is_invalid(archive: Archive) {
        let label = "invalid-label".to_string();
        assert!(matches!(archive.last(&label, 123),
                         Err(Error::UnexpectedLabel(label))));
    }

    #[rstest]
    fn test_history_should_be_iterator_of_pushed_metrics(mut archive: Archive) {
        let mut expected_metrics = Vec::new();
        (1..10).for_each(|i| {
            archive.push("label", 123, Metric::Bitrate(i));
            expected_metrics.push(Metric::Bitrate(i));
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
        self.default.unit()
    }

    fn iter_process(&self, pid: PID) -> Result<Iter<Metric>, Error> {
        Ok(self.series.get(&pid)
            .ok_or(Error::InvalidPID(pid))?
            .iter())
    }

    fn count(&self, pid: PID) -> usize {
        self.series.get(&pid)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    fn default(&self) -> Metric {
        self.default.clone()
    }
}
