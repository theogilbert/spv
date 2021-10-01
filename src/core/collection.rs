use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;

use crate::core::Error;
use crate::core::metrics::Metric;
use crate::core::probe::Probe;
use crate::core::process_view::Pid;
use crate::core::view::{MetricsOverview, MetricView};

pub trait MetricCollector {
    fn collect(&mut self, pids: &[Pid]) -> Result<(), Error>;

    // Same thing as collect, except collected metrics are discarded
    fn calibrate(&mut self, pids: &[Pid]) -> Result<(), Error>;

    fn name(&self) -> &'static str;

    fn view(&self, pid: Pid, resolution: Duration) -> MetricView;
    fn overview(&self) -> MetricsOverview;

    fn compare_pids_by_last_metrics(&self, pid1: Pid, pid2: Pid) -> Ordering;
}

pub struct ProbeCollector<M> where M: Metric + Copy + PartialOrd {
    collection: MetricCollection<M>,
    probe: Box<dyn Probe<M>>,
    default: M,
}

impl<M> ProbeCollector<M> where M: Metric + Copy + PartialOrd {
    pub fn new(probe: impl Probe<M> + 'static) -> Self {
        let default = probe.default_metric();

        Self {
            collection: MetricCollection::<M>::new(probe.default_metric()),
            probe: Box::new(probe),
            default,
        }
    }

    fn process_metrics(&self, pid: Pid) -> Result<Vec<&dyn Metric>, Error> {
        self.collection.metrics(pid)
            .map(|v| v.into_iter().map(|m| m as &dyn Metric).collect())
    }
}

impl<M> MetricCollector for ProbeCollector<M> where M: Metric + Copy + PartialOrd {
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

    fn name(&self) -> &'static str {
        self.probe.name()
    }

    fn view(&self, pid: Pid, resolution: Duration) -> MetricView {
        let metrics = self.process_metrics(pid)
            .unwrap_or_default();

        MetricView::new(metrics, resolution, &self.default)
    }

    fn overview(&self) -> MetricsOverview {
        let last_metrics = self.collection.pids().iter()
            .map(|pid| (**pid, self.collection.last_or_default(**pid) as &dyn Metric))
            .collect();

        MetricsOverview::new(last_metrics, &self.default)
    }

    fn compare_pids_by_last_metrics(&self, pid1: Pid, pid2: Pid) -> Ordering {
        let last_pid1 = self.collection.last_or_default(pid1);
        let last_pid2 = self.collection.last_or_default(pid2);

        last_pid1.partial_cmp(last_pid2)
            .unwrap_or(Ordering::Equal)
    }
}

/// For a given Metric, keep an history of all metric values for all processes
struct MetricCollection<M> where M: Metric + Copy + PartialOrd {
    series: HashMap<Pid, Vec<M>>,
    default: M,
}

impl<M> MetricCollection<M> where M: Metric + Copy + PartialOrd {
    pub fn new(default: M) -> Self {
        Self { series: HashMap::new(), default }
    }

    fn push(&mut self, pid: Pid, metric: M) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new())
        };

        process_series.push(metric);
    }

    fn metrics(&self, pid: Pid) -> Result<Vec<&M>, Error> {
        self.series.get(&pid)
            .map(|v| v.iter().collect())
            .ok_or(Error::InvalidPID(pid))
    }

    fn last_or_default(&self, pid: Pid) -> &M {
        self.metrics(pid)
            .map(|v| v.last().copied().unwrap_or(&self.default))
            .unwrap_or(&self.default)
    }

    fn pids(&self) -> Vec<&Pid> { self.series.keys().collect() }
}