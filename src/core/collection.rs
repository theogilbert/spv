//! Metrics collector and store

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;

use crate::core::Error;
use crate::core::metrics::Metric;
use crate::core::probe::Probe;
use crate::core::process::Pid;
use crate::core::view::{MetricsOverview, MetricView};


/// Types which can collect and store a specific type of [`Metric`](crate::core::metrics::Metric)
///
/// A `MetricCollector` should only collect metrics from a single [`Probe`](crate::core::probe::Probe).
///
/// Each concrete `Metric` type requires its own `MetricCollector` implementation (c.f. [ProbeCollector](struct.ProbeCollector)).<br/>
/// Through this trait, these different implementations can be managed in a generic manner.
pub trait MetricCollector {
    /// Probes metrics for the given processes, and stores them.
    ///
    /// # Arguments
    ///  * `pids`: A slice containing the [`Pids`](crate::core::process::Pid) to probe.
    fn collect(&mut self, pids: &[Pid]) -> Result<(), Error>;

    /// Probes metrics for the given processes, without storing them.
    ///
    /// Some probe implementations require an initial measurement to be calibrated. As this first
    /// metric might not be accurate, we do not store it.
    ///
    /// # Arguments
    ///  * `pids`: A slice containing the [`Pids`](crate::core::process::Pid) to probe.
    fn calibrate(&mut self, pids: &[Pid]) -> Result<(), Error>;

    /// Compares two processes by their last collected metric.
    ///
    /// As it is not possible to compare [`Metric`](crate::core::metrics::Metric) trait objects, we
    /// can rely on this method to sort two processes based on their last metrics. The collector
    /// implementation knows how to sort the metrics as it knows their concrete type.
    ///
    /// # Arguments
    ///  * `pid1`, `pid2`: The ID of the processes to compare
    fn compare_pids_by_last_metrics(&self, pid1: Pid, pid2: Pid) -> Ordering;

    /// Returns a name describing the collected metrics.
    fn name(&self) -> &'static str;

    /// Builds a [`MetricView`](crate::core::view::MetricView), offering insight on the collected
    /// metrics of a given process.
    ///
    /// # Arguments
    ///  * `pid`: The ID of the process for which to get the `MetricView`
    fn view(&self, pid: Pid) -> MetricView;

    /// Builds a [`MetricsOverview`](crate::core::view::MetricsOverview), containing the last metrics
    /// of all running processes.
    fn overview(&self) -> MetricsOverview;
}

/// An implementation of [`MetricCollector`](MetricCollector)
///
/// Uses a [`Probe`](crate::core::probe::Probe) object to probe metrics.
pub struct ProbeCollector<M> where M: Metric + Copy + PartialOrd + Default {
    collection: MetricCollection<M>,
    probe: Box<dyn Probe<M>>,
    resolution: Duration,
}

impl<M> ProbeCollector<M> where M: Metric + Copy + PartialOrd + Default {
    pub fn new(probe: impl Probe<M> + 'static, resolution: Duration) -> Self {
        Self {
            collection: MetricCollection::<M>::new(),
            probe: Box::new(probe),
            resolution,
        }
    }
}

impl<M> MetricCollector for ProbeCollector<M> where M: Metric + Copy + PartialOrd + Default {
    fn collect(&mut self, pids: &[Pid]) -> Result<(), Error> {
        let metrics = self.probe.probe_processes(pids)?;

        for (pid, m) in metrics.into_iter() {
            self.collection.push(pid, m)
        }

        Ok(())
    }

    fn calibrate(&mut self, pids: &[Pid]) -> Result<(), Error> {
        self.probe.probe_processes(pids)
            .map(|_| ())
    }

    fn compare_pids_by_last_metrics(&self, pid1: Pid, pid2: Pid) -> Ordering {
        let last_pid1 = self.collection.last_or_default(pid1);
        let last_pid2 = self.collection.last_or_default(pid2);

        last_pid1.partial_cmp(last_pid2)
            .unwrap_or(Ordering::Equal)
    }

    fn name(&self) -> &'static str {
        self.probe.name()
    }

    fn view(&self, pid: Pid) -> MetricView { self.collection.view(pid, self.resolution) }

    fn overview(&self) -> MetricsOverview { self.collection.overview() }
}

/// MetricCollection stores concrete metrics.<br/>
/// It can also return them as &dyn Metric through MetricView or MetricsOverview.
///
/// This struct was created to move the metric store logic out of MetricCollector
pub(crate) struct MetricCollection<M> where M: Metric + Copy + PartialOrd + Default {
    series: HashMap<Pid, Vec<M>>,
    default: M,
}

impl<M> MetricCollection<M> where M: Metric + Copy + PartialOrd + Default {
    pub fn new() -> Self {
        Self { series: HashMap::new(), default: M::default() }
    }

    pub fn push(&mut self, pid: Pid, metric: M) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new())
        };

        process_series.push(metric);
    }

    pub fn metrics(&self, pid: Pid) -> Result<Vec<&M>, Error> {
        self.series.get(&pid)
            .map(|v| v.iter().collect())
            .ok_or(Error::InvalidPID(pid))
    }

    pub fn last_or_default(&self, pid: Pid) -> &M {
        self.metrics(pid)
            .map(|v| v.last().copied().unwrap_or(&self.default))
            .unwrap_or(&self.default)
    }

    pub fn pids(&self) -> Vec<&Pid> { self.series.keys().collect() }

    pub fn view(&self, pid: Pid, resolution: Duration) -> MetricView {
        let metrics = self.get_metrics_as_metric_trait_objects(pid)
            .unwrap_or_default();

        MetricView::new(metrics, resolution, &self.default)
    }

    fn get_metrics_as_metric_trait_objects(&self, pid: Pid) -> Result<Vec<&dyn Metric>, Error> {
        self.metrics(pid)
            .map(|v| v.into_iter().map(|m| m as &dyn Metric).collect())
    }

    pub fn overview(&self) -> MetricsOverview {
        let last_metrics = self.pids().iter()
            .map(|pid| (**pid, self.last_or_default(**pid) as &dyn Metric))
            .collect();

        MetricsOverview::new(last_metrics, &self.default)
    }
}