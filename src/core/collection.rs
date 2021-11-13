//! Metrics collector and store

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::core::iteration::{Iteration, Span};
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
    /// * `current_iteration`: The current program iteration at which this method is called
    fn collect(&mut self, pids: &[Pid], current_iteration: Iteration) -> Result<(), Error>;

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

impl<M: 'static> ProbeCollector<M>
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

impl<M: 'static> MetricCollector for ProbeCollector<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    fn collect(&mut self, pids: &[Pid], current_iteration: Iteration) -> Result<(), Error> {
        let metrics = self.probe.probe_processes(pids)?;

        for (pid, m) in metrics.into_iter() {
            self.collection.push(pid, m, current_iteration);
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
        return_map: HashMap<Pid, f64>,
    }

    impl Probe<PercentMetric> for ProbeFake {
        fn name(&self) -> &'static str {
            return "fake";
        }

        fn probe(&mut self, pid: Pid) -> Result<PercentMetric, Error> {
            Ok(PercentMetric::new(self.return_map.get(&pid).copied().unwrap()))
        }
    }

    fn create_collector_with_map(return_map: HashMap<Pid, f64>) -> ProbeCollector<PercentMetric> {
        let probe = ProbeFake { return_map };
        ProbeCollector::new(probe)
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
        collector.collect(&[1, 2], 0).unwrap();

        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), expected_ord);
    }

    #[test]
    fn test_collector_should_compare_pids_as_equal_when_both_have_not_been_collected() {
        let collector = create_collector_with_map(hashmap!());

        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), Ordering::Equal);
    }

    #[test]
    fn test_collector_should_compare_collected_process_metric_to_default_when_other_process_has_not_been_collected() {
        let return_map = hashmap!(1 => 50.);
        let mut collector = create_collector_with_map(return_map);
        collector.collect(&[1], 0).unwrap();

        // Pid 1 should be Ordering::Greater than Pid 2 with 50% > default=0%
        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), Ordering::Greater);
    }

    #[rstest]
    fn test_process_metrics_should_be_empty_when_not_collected() {
        let process_data = ProcessMetadata::new(1, 0, "cmd");
        let collector = create_collector_with_map(hashmap!());

        let view = collector.view(&process_data, Span::new(0, 59));

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_be_empty_when_only_calibrated() {
        let process_data = ProcessMetadata::new(1, 0, "cmd");
        let mut collector = create_collector_with_map(hashmap!(1 => 10.));
        collector.calibrate(&[1]).unwrap();

        let view = collector.view(&process_data, Span::new(0, 59));

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_not_be_empty_when_collected() {
        let process_data = ProcessMetadata::new(1, 0, "cmd");
        let mut collector = create_collector_with_map(hashmap!(1 => 10.));
        collector.collect(&[1], 0).unwrap();

        let view = collector.view(&process_data, Span::new(0, 59));

        assert_eq!(view.as_slice(), &[&PercentMetric::new(10.) as &dyn Metric]);
    }
}

/// MetricCollection manages ProcessData instances to store processes' metrics.<br/>
pub(crate) struct MetricCollection<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    processes_data: HashMap<Pid, ProcessData<M>>,
    default: M,
}

impl<M: 'static> MetricCollection<M>
where
    M: Metric + Copy + PartialOrd + Default,
{
    pub fn new() -> Self {
        Self {
            processes_data: HashMap::new(),
            default: M::default(),
        }
    }

    pub fn push(&mut self, pid: Pid, metric: M, current_iteration: Iteration) {
        let process_data = match self.processes_data.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(ProcessData::new(current_iteration)),
        };

        process_data.push(metric);
    }

    pub fn last_or_default(&self, pid: Pid) -> &M {
        self.processes_data
            .get(&pid)
            .map(|pd| pd.last())
            .flatten()
            .unwrap_or(&self.default)
    }

    pub fn view(&self, pm: &ProcessMetadata, span: Span) -> MetricView {
        self.processes_data
            .get(&pm.pid())
            .map(|pd| pd.view(span))
            .unwrap_or_else(|| Self::build_default_view(pm, span))
    }

    fn build_default_view<'a, 'b>(pm: &'a ProcessMetadata, span: Span) -> MetricView<'b> {
        MetricView::new(
            vec![],
            Box::new(M::default()) as Box<dyn Metric>,
            span,
            pm.running_span().begin(),
        )
    }

    pub fn overview(&self) -> MetricsOverview {
        let last_metrics = self
            .processes_data
            .keys()
            .copied()
            .map(|pid| (pid, self.last_or_default(pid) as &dyn Metric))
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
        collection.push(1, PercentMetric::new(1.), 0);
        collection.push(1, PercentMetric::new(2.), 1);

        assert_eq!(collection.last_or_default(1), &PercentMetric::new(2.));
    }
}

/// ProcessData is the private structure which actually stores the concrete metrics of a process
pub(crate) struct ProcessData<M>
where
    M: Metric + Default,
{
    metrics: Vec<M>,
    first_iteration: Iteration,
}

impl<M: 'static> ProcessData<M>
where
    M: Metric + Default,
{
    pub fn new(first_metric_iteration: Iteration) -> Self {
        Self {
            metrics: vec![],
            first_iteration: first_metric_iteration,
        }
    }

    pub fn push(&mut self, metric: M) {
        self.metrics.push(metric);
    }

    pub fn last(&self) -> Option<&M> {
        self.metrics.last()
    }

    pub fn view(&self, span: Span) -> MetricView {
        let metrics = self.extract_metrics_according_to_span(&span);
        let default = Box::new(M::default()) as Box<dyn Metric>;
        MetricView::new(metrics, default, span, self.first_iteration)
    }

    fn extract_metrics_according_to_span(&self, span: &Span) -> Vec<&dyn Metric> {
        let expired_metrics_count = span.begin().checked_sub(self.first_iteration).unwrap_or(usize::MIN);

        self.metrics
            .iter()
            .skip(expired_metrics_count)
            .take(span.size())
            .map(|m| m as &dyn Metric)
            .collect()
    }
}

#[cfg(test)]
mod test_process_data {
    use rstest::*;

    use crate::core::collection::ProcessData;
    use crate::core::iteration::Span;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::process::ProcessMetadata;

    fn build_process_data_and_push(metrics: &[f64]) -> ProcessData<PercentMetric> {
        let mut process_data = ProcessData::new(0);
        metrics
            .into_iter()
            .for_each(|v| process_data.push(PercentMetric::new(*v)));

        process_data
    }

    fn assert_view_slice_equals_percent_metrics_slice(view_slice: &[&dyn Metric], percent_values: &[f64]) {
        let percent_metrics: Vec<PercentMetric> =
            percent_values.iter().copied().map(|v| PercentMetric::new(v)).collect();

        let dyn_metrics_slice: Vec<&dyn Metric> = percent_metrics.iter().map(|p| p as &dyn Metric).collect();

        assert_eq!(view_slice, &dyn_metrics_slice);
    }

    #[fixture]
    fn process_metadata() -> ProcessMetadata {
        ProcessMetadata::new(2, 0, "command")
    }

    #[rstest]
    fn test_view_should_be_empty_by_default() {
        let process_data = ProcessData::<PercentMetric>::new(0);
        let view = process_data.view(Span::new(0, 59));

        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[]);
    }

    #[rstest]
    fn test_view_should_include_metrics_in_span() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);
        let view = process_data.view(Span::new(0, 3));

        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[0., 1., 2., 3.]);
    }

    #[rstest]
    fn test_too_old_metrics_should_not_be_in_view_when_process_has_few_metrics() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);
        let view = process_data.view(Span::new(2, 3));

        // with the specified span (begin=2), only the metrics 2 and 3 should be exported in the view
        // as process_metadata has a spawn_iteration value of 0 (first metric created at iteration 0)
        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[2., 3.]);
    }

    #[rstest]
    fn test_extract_should_only_return_1_metric_if_span_covers_1_iteration() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);
        let view = process_data.view(Span::new(1, 1));

        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[1.]);
    }

    #[rstest]
    fn test_extract_should_only_return_2_metrics_if_span_covers_2_iterations() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);
        let view = process_data.view(Span::new(2, 3));

        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[2., 3.]);
    }

    #[rstest]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);
        let view = process_data.view(Span::new(0, 59));

        assert_view_slice_equals_percent_metrics_slice(view.as_slice(), &[0., 1., 2., 3.]);
    }

    #[rstest]
    fn test_max_f64_should_not_return_values_out_of_span() {
        let process_data = build_process_data_and_push(&[10., 0., 2.]);
        let view = process_data.view(Span::new(1, 2));

        assert_eq!(view.max_f64(), 2.);
    }
}
