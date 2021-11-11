//! Metrics collector and store

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::core::iteration::Span;
use crate::core::metrics::Metric;
use crate::core::probe::Probe;
use crate::core::process::{Pid, ProcessMetadata};
use crate::core::view::{MetricView, MetricsOverview};
use crate::core::Error;

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
    /// As we do not allow comparison between [`Metric`](crate::core::metrics::Metric) trait objects, we
    /// can rely on this method to sort two processes based on their last metrics. A given collector
    /// instance knows how to compare metrics as it knows their concrete type.
    ///
    /// If one of the process has no collected metrics yet, the metric used for the comparison will
    /// be the default metric.
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
    ///  * `pm`: The process for which to view metrics
    ///  * `span`: The span of iterations covered by the metric view
    fn view(&self, pm: &ProcessMetadata, span: Span) -> MetricView;

    /// Builds a [`MetricsOverview`](crate::core::view::MetricsOverview), containing the last metrics
    /// of all running processes.
    fn overview(&self) -> MetricsOverview;
}

/// An implementation of [`MetricCollector`](MetricCollector)
///
/// Uses a [`Probe`](crate::core::probe::Probe) object to probe metrics.
pub struct ProbeCollector<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    collection: MetricCollection<M>,
    probe: Box<dyn Probe<M>>,
}

impl<M> ProbeCollector<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    pub fn new(probe: impl Probe<M> + 'static) -> Self {
        Self {
            collection: MetricCollection::<M>::new(),
            probe: Box::new(probe),
        }
    }
}

impl<M> MetricCollector for ProbeCollector<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    fn collect(&mut self, pids: &[Pid]) -> Result<(), Error> {
        let metrics = self.probe.probe_processes(pids)?;

        for (pid, m) in metrics.into_iter() {
            self.collection.push(pid, m);
        }

        Ok(())
    }

    fn calibrate(&mut self, pids: &[Pid]) -> Result<(), Error> {
        self.probe.probe_processes(pids).map(|_| ())
    }

    fn compare_pids_by_last_metrics(&self, pid1: Pid, pid2: Pid) -> Ordering {
        let last_pid1 = self.collection.last_or_default(pid1);
        let last_pid2 = self.collection.last_or_default(pid2);

        last_pid1.partial_cmp(last_pid2).unwrap_or(Ordering::Equal)
    }

    fn name(&self) -> &'static str {
        self.probe.name()
    }

    fn view(&self, pm: &ProcessMetadata, span: Span) -> MetricView {
        self.collection.view(pm, span)
    }

    fn overview(&self) -> MetricsOverview {
        self.collection.overview()
    }
}

#[cfg(test)]
mod test_probe_collector {
    use std::cmp::Ordering;
    use std::collections::HashMap;

    use rstest::*;

    use crate::core::collection::{MetricCollector, ProbeCollector};
    use crate::core::iteration::Span;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::probe::Probe;
    use crate::core::process::{Pid, ProcessMetadata};
    use crate::core::Error;

    struct ProbeFake {
        return_map: Option<HashMap<Pid, f64>>,
        sequence: Option<Vec<f64>>,
    }

    impl Probe<PercentMetric> for ProbeFake {
        fn name(&self) -> &'static str {
            return "fake";
        }

        fn probe(&mut self, pid: Pid) -> Result<PercentMetric, Error> {
            if let Some(return_map) = self.return_map.as_ref() {
                Ok(PercentMetric::new(*return_map.get(&pid).unwrap()))
            } else if let Some(sequence) = self.sequence.as_mut() {
                Ok(PercentMetric::new(sequence.pop().unwrap()))
            } else {
                Ok(PercentMetric::new(1.))
            }
        }
    }

    fn create_collector() -> ProbeCollector<PercentMetric> {
        let probe = ProbeFake {
            return_map: None,
            sequence: None,
        };
        ProbeCollector::new(probe)
    }

    fn create_collector_with_map(return_map: HashMap<Pid, f64>) -> ProbeCollector<PercentMetric> {
        let probe = ProbeFake {
            return_map: Some(return_map),
            sequence: None,
        };
        ProbeCollector::new(probe)
    }

    fn create_collector_with_sequence_and_collect(pid: Pid, mut sequence: Vec<f64>) -> ProbeCollector<PercentMetric> {
        let sequence_len = sequence.len();
        sequence.reverse();

        let probe = ProbeFake {
            return_map: None,
            sequence: Some(sequence),
        };
        let mut collector = ProbeCollector::new(probe);

        for _ in 0..sequence_len {
            collector.collect(&[pid]).unwrap();
        }

        collector
    }

    #[fixture]
    fn process_metadata() -> ProcessMetadata {
        ProcessMetadata::new(2, 0, "command")
    }

    #[rstest]
    fn test_collector_should_be_empty_by_default(process_metadata: ProcessMetadata) {
        let collector = create_collector();
        let view = collector.view(&process_metadata, Span::from_end_and_size(60, 60));

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_be_empty_when_only_calibrated(process_metadata: ProcessMetadata) {
        let mut collector = create_collector();
        collector.calibrate(&[1, 2, 3]).unwrap();

        let view = collector.view(&process_metadata, Span::from_end_and_size(60, 60));

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_process_metrics_should_be_empty_when_process_has_not_been_collected(process_metadata: ProcessMetadata) {
        let mut collector = create_collector();
        collector.collect(&[1]).unwrap();

        let view = collector.view(&process_metadata, Span::from_end_and_size(60, 60));

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_not_be_empty_when_metrics_collected(process_metadata: ProcessMetadata) {
        let return_map = hashmap!(1 => 50., 2 => 45.);
        let mut collector = create_collector_with_map(return_map);
        collector.collect(&[1, 2]).unwrap();

        let view = collector.view(&process_metadata, Span::from_end_and_size(60, 60));
        let extract = view.as_slice();

        assert_eq!(extract.len(), 1);
        assert_eq!(extract[0], &PercentMetric::new(45.));
    }

    #[rstest]
    fn test_extract_should_return_only_last_metric_if_span_coveres_1_iteration(process_metadata: ProcessMetadata) {
        let collector = create_collector_with_sequence_and_collect(process_metadata.pid(), vec![0., 1.]);
        let view = collector.view(&process_metadata, Span::from_end_and_size(1, 1));

        assert_eq!(view.as_slice(), &[view.last_or_default()]);
    }

    #[rstest]
    fn test_extract_should_return_two_last_metric_if_span_covers_2_iterations(process_metadata: ProcessMetadata) {
        let collector = create_collector_with_sequence_and_collect(process_metadata.pid(), vec![0., 1., 2., 3.]);
        let view = collector.view(&process_metadata, Span::from_end_and_size(3, 2));

        let expected: &[&dyn Metric; 2] = &[&PercentMetric::new(2.), &PercentMetric::new(3.)];

        assert_eq!(view.as_slice(), expected);
    }

    #[rstest]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count(process_metadata: ProcessMetadata) {
        let collector = create_collector_with_sequence_and_collect(process_metadata.pid(), vec![0., 1., 2., 3.]);
        let view = collector.view(&process_metadata, Span::from_end_and_size(60, 60));
        let extract = view.as_slice();

        assert_eq!(extract.len(), 4);
        assert_eq!(extract[0], &PercentMetric::new(0.));
        assert_eq!(extract[3], &PercentMetric::new(3.));
    }

    #[rstest]
    fn test_max_f64_should_not_return_values_out_of_span(process_metadata: ProcessMetadata) {
        let collector = create_collector_with_sequence_and_collect(process_metadata.pid(), vec![10., 0., 2.]);
        let view = collector.view(&process_metadata, Span::from_end_and_size(2, 2));

        assert_eq!(view.max_f64(), 2.);
    }

    #[rstest]
    #[case(50., 45., Ordering::Greater)]
    #[case(50., 55., Ordering::Less)]
    #[case(50., 50., Ordering::Equal)]
    fn test_collector_should_correctly_compare_pids(
        #[case] metric_pid1: f64,
        #[case] metric_pid2: f64,
        #[case] expected_ord: Ordering,
    ) {
        let return_map = hashmap!(1 => metric_pid1, 2 => metric_pid2);
        let mut collector = create_collector_with_map(return_map);
        collector.collect(&[1, 2]).unwrap();

        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), expected_ord);
    }

    #[test]
    fn test_collector_should_compare_pid_as_equal_when_not_collected() {
        let return_map = hashmap!(1 => 50., 2 => 45.);
        let mut collector = create_collector_with_map(return_map);
        collector.calibrate(&[1, 2]).unwrap(); // we calibrate here, we do not collect

        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), Ordering::Equal);
    }

    #[test]
    fn test_collector_should_compare_process_metric_to_default_metric_when_other_process_has_not_been_collected() {
        let return_map = hashmap!(1 => 50.);
        let mut collector = create_collector_with_map(return_map);
        collector.collect(&[1]).unwrap();

        // Pid 1 should be Ordering::Greater than Pid 2 with 50% > default=0%
        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), Ordering::Greater);
    }
}

/// MetricCollection stores concrete metrics.<br/>
/// It can also return them as &dyn Metric through MetricView or MetricsOverview.
///
/// This struct was created to move the metric store logic out of MetricCollector
pub(crate) struct MetricCollection<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    series: HashMap<Pid, Vec<M>>,
    default: M,
}

impl<M> MetricCollection<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    pub fn new() -> Self {
        Self {
            series: HashMap::new(),
            default: M::default(),
        }
    }

    pub fn push(&mut self, pid: Pid, metric: M) {
        let process_series = match self.series.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Vec::new()),
        };

        process_series.push(metric);
    }

    pub fn last_or_default(&self, pid: Pid) -> &M {
        self.metrics_of_process(pid)
            .map(|v| v.last().copied().unwrap_or(&self.default))
            .unwrap_or(&self.default)
    }

    fn metrics_of_process(&self, pid: Pid) -> Result<Vec<&M>, Error> {
        self.series
            .get(&pid)
            .map(|v| v.iter().collect())
            .ok_or(Error::InvalidPID(pid))
    }

    pub fn view(&self, pm: &ProcessMetadata, span: Span) -> MetricView {
        let metrics = self.extract_metrics_according_to_span(pm.pid(), &span);
        MetricView::new(metrics, &self.default, span, pm.running_span().begin())
    }

    fn extract_metrics_according_to_span(&self, pid: Pid, span: &Span) -> Vec<&dyn Metric> {
        if let Ok(metrics) = self.metrics_of_process(pid) {
            let collected_metrics_count = metrics.len();
            let expected_metrics_count = span.size();
            let skipped_metrics = collected_metrics_count.saturating_sub(expected_metrics_count);

            metrics
                .into_iter()
                .skip(skipped_metrics)
                .map(|m| m as &dyn Metric)
                .collect()
        } else {
            vec![]
        }
    }

    pub fn overview(&self) -> MetricsOverview {
        let last_metrics = self
            .series
            .keys()
            .map(|pid| (*pid, self.last_or_default(*pid) as &dyn Metric))
            .collect();

        MetricsOverview::new(last_metrics, &self.default)
    }
}

#[cfg(test)]
mod test_metric_collection {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::PercentMetric;

    #[test]
    fn test_should_return_default_when_no_metric() {
        let collection = MetricCollection::<PercentMetric>::new();
        assert_eq!(collection.last_or_default(1), &PercentMetric::default());
    }

    #[test]
    fn test_should_return_last_metric_when_has_metric() {
        let mut collection = MetricCollection::<PercentMetric>::new();
        collection.push(1, PercentMetric::new(1.));
        collection.push(1, PercentMetric::new(2.));

        assert_eq!(collection.last_or_default(1), &PercentMetric::new(2.));
    }
}
