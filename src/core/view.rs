//! View of collected metrics

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::core::metrics::{DatedMetric, Metric};
use crate::core::process::{Pid, ProcessMetadata};
use crate::core::time::Span;

/// Snapshot of all collected metrics of a single process, from a single probe
///
/// Refer to the [`MetricCollector`](crate::core::collection::MetricCollector) trait to instanciate a `MetricView`
pub struct MetricView<'a> {
    dated_metrics: Vec<DatedMetric<'a>>,
    default: Box<dyn Metric>,
    span: Span,
}

impl<'a> MetricView<'a> {
    pub(crate) fn new(dated_metrics: Vec<DatedMetric<'a>>, default: Box<dyn Metric>, span: Span) -> Self {
        Self {
            dated_metrics,
            default,
            span,
        }
    }

    /// Returns a slice of the metrics contained in this view.
    /// The slice only covers the last metrics covered by the `span` parameter.
    pub fn as_slice(&'a self) -> &[DatedMetric<'a>] {
        &self.dated_metrics
    }

    /// Returns the unit representation of the metrics contained in this view
    pub fn unit(&self) -> &'static str {
        self.default.unit()
    }

    /// Returns the latest collected metric, or its default value if no metric has
    /// been collected for this process.
    pub fn last_or_default(&self) -> &dyn Metric {
        self.dated_metrics.last().map(|dm| dm.metric).unwrap_or(&*self.default)
    }

    /// Returns the greatest f64 value of the metric in the given span. See [`MetricView::new()`](#method.extract) for
    /// the behavior of `span`.
    ///
    /// If the metrics have a cardinality greater than one, the max f64 component of the metric is
    /// used for the comparison.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago the metrics should be compared
    pub fn max_f64(&self) -> f64 {
        self.max_metric().max_value()
    }

    /// Returns a concise representation of the greatest metric in the given span. See [`MetricView::new()`](#method.extract) for
    /// the behavior of `span`.
    ///
    /// If the metrics have a cardinality greater than one, the max f64 component of the metric is
    /// used for the comparison.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago the metrics should be compared
    pub fn concise_repr_of_value(&self, value: f64) -> String {
        self.default.concise_repr_of_value(value)
    }

    fn max_metric(&self) -> &dyn Metric {
        self.dated_metrics
            .iter()
            .map(|dm| dm.metric)
            .max_by(|m1, m2| {
                let v1 = m1.max_value();
                let v2 = m2.max_value();

                v1.partial_cmp(&v2).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(&*self.default)
    }

    pub fn span(&self) -> &Span {
        &self.span
    }
}

#[cfg(test)]
mod test_metric_view {
    use std::time::Duration;

    use rstest::*;

    use crate::core::metrics::{DatedMetric, IOMetric, Metric, PercentMetric};
    use crate::core::time::{Span, Timestamp};
    use crate::core::view::MetricView;

    #[fixture]
    fn metrics() -> Vec<PercentMetric> {
        vec![
            PercentMetric::new(10.),
            PercentMetric::new(20.),
            PercentMetric::new(15.),
        ]
    }

    fn percents_to_dated_metrics(metrics: &Vec<PercentMetric>) -> Vec<DatedMetric> {
        let now = Timestamp::now();
        metrics
            .iter()
            .enumerate()
            .map(|(idx, m)| DatedMetric::new(m as &dyn Metric, now + Duration::from_secs(idx as u64)))
            .collect()
    }

    #[fixture]
    fn default() -> Box<dyn Metric> {
        Box::new(PercentMetric::default()) as Box<dyn Metric>
    }

    #[fixture]
    fn span() -> Span {
        Span::new(Timestamp::now(), Timestamp::now() + Duration::from_secs(10))
    }

    #[rstest]
    fn test_last_or_default_should_be_latest_metric(metrics: Vec<PercentMetric>, default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(percents_to_dated_metrics(&metrics), default, span);

        assert_eq!(view.last_or_default(), metrics.last().unwrap());
    }

    #[rstest]
    fn test_last_or_default_should_be_default_when_view_is_empty(default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(vec![], default, span);

        assert_eq!(view.last_or_default(), &PercentMetric::default());
    }

    #[rstest]
    fn test_unit_should_be_metric_unit(default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(vec![], default, span);

        assert_eq!(view.unit(), PercentMetric::default().unit());
    }

    #[rstest]
    fn test_max_f64_should_return_max_value(metrics: Vec<PercentMetric>, default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(percents_to_dated_metrics(&metrics), default, span);

        assert_eq!(view.max_f64(), 20.);
    }

    #[rstest]
    fn test_max_f64_should_return_default_f64_when_empty(default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(vec![], default, span);

        assert_eq!(view.max_f64(), PercentMetric::default().as_f64(0).unwrap());
    }

    #[rstest]
    fn test_concise_repr_should_return_repr_of_default_metric(span: Span) {
        let default = Box::new(IOMetric::default()) as Box<dyn Metric>;
        let view = MetricView::new(vec![], default, span);

        assert_eq!(view.concise_repr_of_value(2048.), "2.0k".to_string());
    }

    #[rstest]
    fn test_should_return_correct_span(default: Box<dyn Metric>, span: Span) {
        let view = MetricView::new(vec![], default, span);

        assert_eq!(view.span(), &span);
    }
}

/// Overview of a single probe's latest metrics, for all running processes
///
/// Refer to the [`MetricCollector`](crate::core::collection::MetricCollector) trait to instanciate a `MetricsOverview`
pub struct MetricsOverview<'a> {
    last_metrics: HashMap<Pid, &'a dyn Metric>,
    default: &'a dyn Metric,
}

impl<'a> MetricsOverview<'a> {
    pub fn new(last_metrics: HashMap<Pid, &'a dyn Metric>, default: &'a dyn Metric) -> Self {
        Self { last_metrics, default }
    }

    /// Returns the latest collected `Metric` for a given process
    /// If no metric has been collected for this process, the default `Metric` value is returned.
    ///
    /// # Arguments
    ///  * pid: The ID of the process
    pub fn last_or_default(&self, pid: Pid) -> &dyn Metric {
        *(self.last_metrics.get(&pid).unwrap_or(&self.default))
    }

    /// Returns the unit representation of the metrics contained in this view
    pub fn unit(&self) -> &'static str {
        self.default.unit()
    }
}

#[cfg(test)]
mod test_metric_overview {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::process::Pid;
    use crate::core::view::MetricsOverview;

    fn build_overview(collection: &MetricCollection<PercentMetric>) -> MetricsOverview {
        collection.overview()
    }

    /// Return collection of PercentMetric containing metrics for `proc_count` processes.<br/>
    /// The Pid values range from `0` to `proc_count - 1`<br/>
    /// To each processes are associated the same PercentMetric values, ranging from `0` to `metrics_count`
    fn produce_metrics_collection(proc_count: usize, values: Vec<f64>) -> MetricCollection<PercentMetric> {
        let mut collection = MetricCollection::new();

        for value in values.into_iter() {
            for proc_idx in 0..proc_count {
                collection.push(proc_idx as Pid, PercentMetric::new(value));
            }
        }

        collection
    }

    #[test]
    fn test_unit_should_be_default_metric_unit() {
        let collection = produce_metrics_collection(2, vec![0., 1.]);
        let overview = build_overview(&collection);

        assert_eq!(overview.unit(), PercentMetric::default().unit());
    }

    #[test]
    fn test_last_or_default_should_return_last_when_proc_has_metrics() {
        let collection = produce_metrics_collection(2, vec![0., 1.]);
        let overview = build_overview(&collection);

        assert_eq!(overview.last_or_default(0), &PercentMetric::new(1.));
    }

    #[test]
    fn test_last_or_default_should_return_default_when_pid_is_unknown() {
        let collection = produce_metrics_collection(2, vec![0., 1.]);
        let overview = build_overview(&collection);

        assert_eq!(overview.last_or_default(2), &PercentMetric::default());
    }
}

pub struct ProcessView<'a> {
    sorted_processes: &'a [ProcessMetadata],
    selected_index: Option<usize>,
}

impl<'a> ProcessView<'a> {
    /// Creates a view containing the processes to list on the UI, and the selected process if any
    ///
    /// Panics if `selected_index` is out of bound of `sorted_processes`
    pub fn new(sorted_processes: &'a [ProcessMetadata], selected_index: Option<usize>) -> ProcessView<'a> {
        if let Some(selected_index) = selected_index {
            if selected_index >= sorted_processes.len() {
                panic!("Selected process index if out of bound {:?}", selected_index);
            }
        }

        Self {
            sorted_processes,
            selected_index,
        }
    }

    pub fn as_slice(&self) -> &[ProcessMetadata] {
        self.sorted_processes
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_process(&self) -> Option<&ProcessMetadata> {
        self.selected_index.map(|idx| &self.sorted_processes[idx])
    }
}

#[cfg(test)]
mod test_process_view {
    use rstest::*;

    use crate::core::process::ProcessMetadata;
    use crate::core::view::ProcessView;

    #[fixture]
    fn processes() -> Vec<ProcessMetadata> {
        vec![ProcessMetadata::new(1, "cmd_1"), ProcessMetadata::new(2, "cmd_2")]
    }

    #[rstest]
    fn test_should_contain_all_processes_in_slice(processes: Vec<ProcessMetadata>) {
        let view = ProcessView::new(&processes, None);

        assert_eq!(view.as_slice(), &processes);
    }

    #[rstest]
    fn test_should_have_no_selected_process(processes: Vec<ProcessMetadata>) {
        let view = ProcessView::new(&processes, None);

        assert_eq!(view.selected_index(), None);
        assert_eq!(view.selected_process(), None);
    }

    #[rstest]
    fn test_should_return_correct_selected_process(processes: Vec<ProcessMetadata>) {
        let view = ProcessView::new(&processes, Some(1));

        assert_eq!(view.selected_index(), Some(1));
        assert_eq!(view.selected_process(), Some(&processes[1]));
    }

    #[rstest]
    #[should_panic]
    fn test_should_panic_when_index_out_of_bound(processes: Vec<ProcessMetadata>) {
        ProcessView::new(&processes, Some(2));
    }
}
