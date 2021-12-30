//! Metrics collector and store

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::core::metrics::{DatedMetric, Metric};
use crate::core::probe::Probe;
use crate::core::process::Pid;
use crate::core::time::{Span, Timestamp};
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

    /// Cleans up the data allocated to collect the given processes.
    ///
    /// This does not cleanup the already collected data for this process.
    ///
    /// # Arguments
    ///  * `pids`: The IDs of the processes to cleanup
    fn cleanup(&mut self, pids: &[Pid]);

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
    ///  * `pid`: The ID of the process for which to view metrics
    ///  * `span`: The time period covered by the metric view
    fn view(&self, pid: Pid, span: Span) -> MetricView;

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
    fn collect(&mut self, pids: &[Pid]) -> Result<(), Error> {
        let metrics = self.probe.probe_processes(pids)?;

        for (pid, m) in metrics.into_iter() {
            self.collection.push(pid, m);
        }

        Ok(())
    }

    fn cleanup(&mut self, pids: &[Pid]) {
        self.probe.cleanup(pids);
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

    fn view(&self, pid: Pid, span: Span) -> MetricView {
        self.collection.view(pid, span)
    }

    fn overview(&self) -> MetricsOverview {
        self.collection.overview()
    }
}

#[cfg(test)]
mod test_probe_collector {
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::time::Duration;

    use rstest::rstest;

    use crate::core::collection::{MetricCollector, ProbeCollector};
    use crate::core::metrics::PercentMetric;
    use crate::core::probe::fakes::FakeProbe;
    use crate::core::process::Pid;
    use crate::core::time::{Span, Timestamp};

    fn create_collector_with_map(return_map: HashMap<Pid, f64>) -> ProbeCollector<PercentMetric> {
        let probe = FakeProbe::from_percent_map(return_map);
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
        collector.collect(&[1, 2]).unwrap();

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
        collector.collect(&[1]).unwrap();

        // Pid 1 should be Ordering::Greater than Pid 2 with 50% > default=0%
        assert_eq!(collector.compare_pids_by_last_metrics(1, 2), Ordering::Greater);
    }

    #[rstest]
    fn test_process_metrics_should_be_empty_when_not_collected() {
        let collector = create_collector_with_map(hashmap!());

        let span = Span::new(Timestamp::now(), Timestamp::now() + Duration::from_secs(60));
        let view = collector.view(1, span);

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_be_empty_when_only_calibrated() {
        let mut collector = create_collector_with_map(hashmap!(1 => 10.));
        collector.calibrate(&[1]).unwrap();

        let span = Span::new(Timestamp::now(), Timestamp::now() + Duration::from_secs(60));
        let view = collector.view(1, span);

        assert_eq!(view.as_slice().len(), 0);
    }

    #[rstest]
    fn test_collector_should_not_be_empty_when_collected() {
        let mut collector = create_collector_with_map(hashmap!(1 => 10.));
        collector.collect(&[1]).unwrap();

        let span = Span::new(Timestamp::now(), Timestamp::now() + Duration::from_secs(60));
        let view = collector.view(1, span);

        assert_eq!(view.as_slice().len(), 1);
    }
}

/// MetricCollection manages ProcessData instances to store processes' metrics.<br/>
pub(super) struct MetricCollection<M>
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

    pub fn push(&mut self, pid: Pid, metric: M) {
        let process_data = match self.processes_data.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(ProcessData::new()),
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

    pub fn view(&self, pid: Pid, span: Span) -> MetricView {
        self.processes_data
            .get(&pid)
            .map(|pd| pd.view(span))
            .unwrap_or_else(|| Self::build_default_view(span))
    }

    fn build_default_view<'a>(span: Span) -> MetricView<'a> {
        MetricView::new(vec![], Box::new(M::default()) as Box<dyn Metric>, span)
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
        collection.push(1, PercentMetric::new(1.));
        collection.push(1, PercentMetric::new(2.));

        assert_eq!(collection.last_or_default(1), &PercentMetric::new(2.));
    }
}

/// Just like `DatedMetric`, except here the metric type is a concrete type
struct ConcreteDatedMetric<M>
where
    M: Metric,
{
    timestamp: Timestamp,
    metric: M,
}

/// ProcessData is the private structure which actually stores the concrete metrics of a process
pub(crate) struct ProcessData<M>
where
    M: Metric + Default,
{
    metrics: Vec<ConcreteDatedMetric<M>>,
}

impl<M: 'static> ProcessData<M>
where
    M: Metric + Default,
{
    pub fn new() -> Self {
        Self { metrics: vec![] }
    }

    pub fn push(&mut self, metric: M) {
        self.metrics.push(ConcreteDatedMetric {
            timestamp: Timestamp::now(),
            metric,
        });
    }

    pub fn last(&self) -> Option<&M> {
        self.metrics.last().map(|m| &m.metric)
    }

    pub fn view(&self, span: Span) -> MetricView {
        let metrics = self.extract_metrics_according_to_span(&span);
        let default = Box::new(M::default()) as Box<dyn Metric>;
        MetricView::new(metrics, default, span)
    }

    fn extract_metrics_according_to_span(&self, span: &Span) -> Vec<DatedMetric> {
        self.metrics
            .iter()
            .filter(|cdm| span.contains(cdm.timestamp))
            .map(|cdm| DatedMetric::new(&cdm.metric as &dyn Metric, cdm.timestamp))
            .collect()
    }
}

#[cfg(test)]
mod test_process_data {
    use std::time::Duration;

    use rstest::*;

    use crate::core::collection::ProcessData;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::time::test_utils::{
        advance_time_and_refresh_timestamp, setup_fake_clock_to_prevent_substract_overflow,
    };
    use crate::core::time::{Span, Timestamp};
    use crate::core::view::MetricView;

    // Builds PercentMetric values from `metrics`, and push them to the ProcessData instance with a 1s interval between
    // each metrics. The last metric is pushed at Timestamp::now().
    fn build_process_data_and_push(metrics: &[f64]) -> ProcessData<PercentMetric> {
        let mut process_data = ProcessData::new();

        metrics.into_iter().for_each(|v| {
            advance_time_and_refresh_timestamp(Duration::from_secs(1));
            process_data.push(PercentMetric::new(*v));
        });

        process_data
    }

    // Builds PercentMetric instances from `percent_values` and compares them to the metrics in the view
    fn assert_view_metrics_equals_percent_metrics(view: &MetricView, percent_values: &[f64]) {
        let pct_metrics: Vec<PercentMetric> = percent_values.iter().copied().map(|v| PercentMetric::new(v)).collect();

        let pct_dyn_metrics: Vec<&dyn Metric> = pct_metrics.iter().map(|p| p as &dyn Metric).collect();

        let view_metrics: Vec<&dyn Metric> = view.as_slice().iter().map(|dm| dm.metric).collect();

        assert_eq!(view_metrics, pct_dyn_metrics);
    }

    #[rstest]
    fn test_view_should_be_empty_by_default() {
        let process_data = ProcessData::<PercentMetric>::new();

        let span = Span::new(Timestamp::now(), Timestamp::now() + Duration::from_secs(60));
        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[]);
    }

    #[rstest]
    fn test_view_should_include_metrics_in_span() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let span = Span::new(Timestamp::now() - Duration::from_secs(3), Timestamp::now());
        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[0., 1., 2., 3.]);
    }

    #[rstest]
    fn test_should_correctly_date_metrics() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let span = Span::new(Timestamp::now() - Duration::from_secs(3), Timestamp::now());
        let view = process_data.view(span);

        let metrics_dates: Vec<_> = view.as_slice().iter().map(|dm| dm.timestamp).collect();

        assert_eq!(
            metrics_dates,
            vec![
                Timestamp::now() - Duration::from_secs(3),
                Timestamp::now() - Duration::from_secs(2),
                Timestamp::now() - Duration::from_secs(1),
                Timestamp::now(),
            ]
        )
    }

    #[rstest]
    fn test_too_old_metrics_should_not_be_in_view_when_process_has_few_metrics() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let span = Span::new(Timestamp::now() - Duration::from_secs(1), Timestamp::now());
        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[2., 3.]);
    }

    #[rstest]
    fn test_extract_should_only_return_1_metric_if_span_covers_1_iteration() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let second_value_timestamp = Timestamp::now() - Duration::from_secs(2);
        let span = Span::new(second_value_timestamp, second_value_timestamp);

        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[1.]);
    }

    #[rstest]
    fn test_extract_should_only_return_2_metrics_if_span_covers_2_iterations() {
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let span = Span::new(Timestamp::now() - Duration::from_secs(1), Timestamp::now());
        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[2., 3.]);
    }

    #[rstest]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count() {
        setup_fake_clock_to_prevent_substract_overflow();
        let process_data = build_process_data_and_push(&[0., 1., 2., 3.]);

        let span = Span::new(
            Timestamp::now() - Duration::from_secs(60),
            Timestamp::now() + Duration::from_secs(60),
        );
        let view = process_data.view(span);

        assert_view_metrics_equals_percent_metrics(&view, &[0., 1., 2., 3.]);
    }

    #[rstest]
    fn test_max_f64_should_not_return_values_out_of_span() {
        let process_data = build_process_data_and_push(&[10., 0., 2.]);

        let span = Span::new(Timestamp::now() - Duration::from_secs(1), Timestamp::now());
        let view = process_data.view(span);

        assert_eq!(view.max_f64(), 2.);
    }
}
