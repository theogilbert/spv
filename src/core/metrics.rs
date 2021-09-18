//! Metric handling

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;

use log::warn;

use crate::core::Error;
use crate::core::process_view::Pid;

/// A quantified information probed from a process
#[derive(Debug, PartialEq, Clone)]
pub enum Metric {
    /// A percent usage. 100% -> `PercentUsage(100.)`
    PercentUsage(f64),
    /// A bitrate, in bytes per second
    Bitrate(usize),
    /// Input / Output rates, in bytes per second
    IO { input: usize, output: usize },
}


impl Metric {
    /// Indicates the amount of dimensions the metric is composed of
    pub fn cardinality(&self) -> usize {
        match self {
            Metric::PercentUsage(_) => 1,
            Metric::Bitrate(_) => 1,
            Metric::IO { input: _, output: _ } => 2,
        }
    }

    /// Returns a raw value from the metric, as f64
    ///
    /// # Arguments
    ///   * `index`: The dimension from which to retrieve the value of the Metric.
    ///         Must be less than `Metric::cardinality()`
    ///
    /// Returns Error::RawMetricAccessError if `index` is not less than `Metric::cardinality()`
    ///
    /// ```
    /// use spv::core::metrics::Metric;
    ///
    /// let io_metric = Metric::IO {input: 10, output: 100};
    /// assert_eq!(io_metric.raw_as_f64(0).unwrap() as u32, 10);
    /// assert_eq!(io_metric.raw_as_f64(1).unwrap() as u32, 100);
    ///
    /// let percent_metric = Metric::PercentUsage(55.);
    /// assert_eq!(percent_metric.raw_as_f64(0).unwrap() as u32, 55);
    /// ```
    pub fn raw_as_f64(&self, index: usize) -> Result<f64, Error> {
        if index >= self.cardinality() {
            Err(Error::RawMetricAccessError(index, self.cardinality()))
        } else {
            Ok(match self {
                Metric::PercentUsage(pct) => *pct,
                Metric::Bitrate(br) => *br as f64,
                Metric::IO { input, output } => {
                    match index {
                        0 => *input as f64,
                        1 => *output as f64,
                        _ => panic!("Invalid raw value index")
                    }
                }
            })
        }
    }

    /// Returns an iterator over the dimensions of the metric
    ///
    /// ```
    /// use spv::core::metrics::Metric;
    ///
    /// let io_metric = Metric::IO {input: 10, output: 100};
    /// let mut iter = io_metric.raw_iter();
    ///
    /// assert_eq!(iter.next().map(|v| v as u32), Some(10));
    /// assert_eq!(iter.next().map(|v| v as u32), Some(100));
    /// assert_eq!(iter.next(), None);
    ///
    /// ```
    pub fn raw_iter(&self) -> RawMetricIter {
        RawMetricIter { metric: self, cur_index: 0 }
    }

    /// Returns a representation of the base unit of the metric (e.g. `B/s` for bitrates)
    pub fn unit(&self) -> &'static str {
        match self {
            Metric::PercentUsage(_) => "%",
            Metric::Bitrate(_) => "B/s",
            Metric::IO { input: _, output: _ } => "B/s"
        }
    }

    /// A very concise representation of the metric
    /// For multi-dimensional metrics, only displays the greatest raw value
    /// ```
    /// use spv::core::metrics::Metric;
    ///
    /// let io_metric = Metric::IO {input: 10, output: 1024};
    /// assert_eq!(io_metric.concise_repr(), "1.0k");
    /// ```
    pub fn concise_repr(&self) -> String {
        match self {
            Metric::PercentUsage(pct) => format!("{:.1}", pct),
            Metric::Bitrate(br) => {
                Self::formatted_bytes(*br, 1)
            }
            Metric::IO { input, output } => {
                let reported_metric = input.max(output);
                Self::formatted_bytes(*reported_metric, 1)
            }
        }
    }

    /// An explicit representation one raw value of the metric
    ///
    /// # Arguments
    ///   * `index`: The dimension of the raw value for which to get an explicit representation.
    ///
    /// If `index` is not less than [`Metric::cardinality()`](enum.Metric.html#method.cardinality),
    /// this method will raise a `Error::RawMetricAccessError`
    /// ```
    /// use spv::core::metrics::Metric;
    ///
    /// let io_metric = Metric::IO {input: 10, output: 1024 * 1024};
    /// assert_eq!(&io_metric.explicit_repr(0).unwrap(), "Input:  10.00B/s");
    /// assert_eq!(&io_metric.explicit_repr(1).unwrap(), "Output: 1.00MB/s");
    /// ```
    pub fn explicit_repr(&self, index: usize) -> Result<String, Error> {
        if index >= self.cardinality() {
            Err(Error::RawMetricAccessError(index, self.cardinality()))
        } else {
            Ok(match self {
                Metric::PercentUsage(pct) => format!("Usage {:.2}%", pct),
                Metric::Bitrate(br) => {
                    format!("{}B/s", Self::formatted_bytes(*br, 2))
                }
                Metric::IO { input, output } => {
                    match index {
                        0 => format!("Input:  {}B/s", Self::formatted_bytes(*input, 2)),
                        1 => format!("Output: {}B/s", Self::formatted_bytes(*output, 2)),
                        _ => panic!("Invalid raw value index")
                    }
                }
            })
        }
    }

    /// Returns a more readable version of `bytes_val`
    /// `formatted_bytes(1294221)` -> 1.2M
    fn formatted_bytes(bytes_val: usize, precision: usize) -> String {
        if bytes_val == 0 {
            return "0".to_string();
        }

        const METRIC_PREFIXES: [&'static str; 4] = ["", "k", "M", "G"];

        let log = (bytes_val as f64).log(1024.)
            .max(0.).floor() as usize;

        let prefix_index = log.min(METRIC_PREFIXES.len() - 1);

        let simplified = bytes_val as f64 / (1024_usize.pow(log as u32) as f64);

        format!("{:.precision$}{}", simplified, METRIC_PREFIXES[prefix_index], precision = precision)
    }
}

impl PartialOrd for Metric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Metric::PercentUsage(pct_self), Metric::PercentUsage(pct_other)) => {
                pct_self.partial_cmp(pct_other)
            }
            (Metric::Bitrate(br_self), Metric::Bitrate(br_other)) => {
                br_self.partial_cmp(br_other)
            }
            (Metric::IO { input: i1, output: o1 }, Metric::IO { input: i2, output: o2 }) => {
                i1.max(o1).partial_cmp(&i2.max(o2))
            }
            (_, _) => panic!("Comparing incompatible metrics"),
        }
    }
}

/// An iterator over the raw value(s) of a [`Metric`](enum.Metric.html) variant
pub struct RawMetricIter<'a> {
    metric: &'a Metric,
    cur_index: usize,
}

impl<'a> Iterator for RawMetricIter<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.metric.raw_as_f64(self.cur_index).ok();
        self.cur_index += 1;

        raw
    }
}


/// Types which can probe processes for a specific kind of [`Metric`](enum.Metric)
pub trait Probe {
    /// The name of the probe, as displayed in the application tab
    fn name(&self) -> &'static str;

    /// An acceptable default metric returned by this probe
    fn default_metric(&self) -> Metric;

    /// Called on each probe refresh, before all processes are probed
    fn init_iteration(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Probe a given process for a [`Metric`](enum.Metric)
    fn probe(&mut self, pid: Pid) -> Result<Metric, Error>;

    /// Returns a map associating a [`Metric`](enum.Metric) instance to each PID
    ///
    /// If a process is not probed correctly, a default value for the given probe is returned
    /// and a WARNING level log is produced
    ///
    /// # Arguments
    ///  * `pids`: A set of `PIDs` to monitor
    ///
    fn probe_processes(&mut self, pids: &[Pid]) -> Result<HashMap<Pid, Metric>, Error> {
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
                entry.insert(MetricHistory::new(default));
                Ok(self)
            }
        }
    }

    pub fn build(self) -> Archive {
        self.archive
    }
}


/// Container for all collected metrics
pub struct Archive {
    metrics: HashMap<String, MetricHistory>,
    resolution: Duration,
}


impl Archive {
    /// Pushes a new [`Metric`](enum.Metric) to the archive
    /// If the label is invalid, `Error::UnexpectedLabel` will be returned
    /// If the metric variant is incompatible with the label, `Error::InvalidMetricVariant` will be
    ///  returned
    ///
    /// # Arguments
    ///  * `label` The name of the label of the metric
    ///  * `pid` The ID of the process from which comes the [`Metric`](enum.Metric)
    ///  * `metric` The new metric to associate to the given process and label
    ///                 Only one variant of [`Metric`](enum.Metric) is allowed per label
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn push(&mut self, label: &str, pid: Pid, metric: Metric) -> Result<(), Error> {
        let pm = match self.metrics.get_mut(label) {
            Some(pm) => Ok(pm),
            None => Err(Error::UnexpectedLabel(label.to_string()))
        }?;

        match (pm.last(pid), metric) {
            (&Metric::PercentUsage(_), Metric::PercentUsage(pct)) => pm.push(pid, Metric::PercentUsage(pct)),
            (&Metric::Bitrate(_), Metric::Bitrate(br)) => pm.push(pid, Metric::Bitrate(br)),
            (&Metric::IO { input: _, output: _ }, Metric::IO { input, output }) => pm.push(pid, Metric::IO { input, output }),
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
    pub fn last(&self, label: &str, pid: Pid) -> Result<&Metric, Error> {
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

    /// Returns an iterator over [`Metric`](enum.Metric) for the given probe label and process ID.
    /// The iterator only contains metrics in the given span
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the unit
    ///  * `pid`: The ID of the process for which to retrieve the history
    ///  * `span`: Indicates metrics over how long to return
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn history(&self, label: &str, pid: Pid, span: Duration) -> Result<&[Metric], Error> {
        let proc_metrics = self.metrics.get(label)
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))?;

        let metrics_count = proc_metrics.count(pid);
        let collected_metrics = self.expected_metrics(span);
        let skipped_metrics = metrics_count.saturating_sub(collected_metrics);

        Ok(&proc_metrics.metrics(pid)?[skipped_metrics..])
    }

    /// Returns the expected step between each metric
    pub fn step(&self) -> Duration {
        self.resolution.clone()
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

    /// Get the default [`Metric`](enum.Metric) associated to the probe with the given label
    ///
    /// # Arguments
    ///  * `label`: The name of the label of the probe for which to retrieve the default [`Metric`](enum.Metric)
    ///
    /// If `label` is invalid, returns a Error::UnexpectedLabel
    ///
    pub fn default_metric(&self, label: &str) -> Result<Metric, Error> {
        self.metrics.get(label)
            .map(|pm| pm.default())
            .ok_or_else(|| Error::UnexpectedLabel(label.to_string()))
    }
}



/// For a given Metric, keep an history of all metric values for all processes
struct MetricHistory {
    default: Metric,
    series: HashMap<Pid, Vec<Metric>>,
}

impl MetricHistory {
    fn new(default: Metric) -> Self {
        Self { default, series: HashMap::new() }
    }

    fn push(&mut self, pid: Pid, metric: Metric) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new())
        };

        process_series.push(metric);
    }

    fn last(&self, pid: Pid) -> &Metric {
        self.series.get(&pid)
            .and_then(|v| v.last())
            .unwrap_or(&self.default)
    }

    fn unit(&self) -> &'static str {
        self.default.unit()
    }

    fn metrics(&self, pid: Pid) -> Result<&[Metric], Error> {
        Ok(self.series.get(&pid)
            .ok_or(Error::InvalidPID(pid))?)
    }

    fn count(&self, pid: Pid) -> usize {
        self.series.get(&pid)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    fn default(&self) -> Metric {
        self.default.clone()
    }
}


#[cfg(test)]
mod test_probe_trait {
    use std::collections::HashMap;

    use rstest::*;

    use crate::core::Error;
    use crate::core::metrics::{Metric, Probe};
    use crate::core::process_view::Pid;

    struct FakeProbe {
        probe_responses: HashMap<Pid, Metric>
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

        assert!(matches!(archive.push(&label, 123, Metric::PercentUsage(45.1)),
                         Err(Error::InvalidMetricVariant(label, _))));
    }

    #[rstest]
    fn test_push_should_fail_when_additional_variant_is_invalid(mut archive: Archive) {
        let label = "label".to_string();
        archive.push(&label, 123, Metric::Bitrate(45))
            .unwrap();

        assert!(matches!(archive.push(&label, 123, Metric::PercentUsage(50.)),
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

        let history = archive.history("label", 123, Duration::from_secs(60))
            .unwrap();

        assert_eq!(&history, &expected_metrics);
    }

    #[rstest]
    fn test_history_with_same_span_and_resolution(filled_archive: Archive, metrics: Vec<Metric>) {
        let history = filled_archive.history("label", 123,
                                             filled_archive.resolution)
            .unwrap();

        assert_eq!(history, &[metrics.last().unwrap().clone()]);
    }

    #[rstest]
    fn test_history_with_double_span_than_resolution(filled_archive: Archive,
                                                     metrics: Vec<Metric>) {
        let history = filled_archive.history("label", 123,
                                             filled_archive.resolution * 2)
            .unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0], metrics[metrics.len() - 2]);
        assert_eq!(history[1], metrics[metrics.len() - 1]);
    }
}