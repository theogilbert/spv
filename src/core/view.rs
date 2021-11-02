//! View of collected metrics

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;

#[cfg(not(test))]
use std::time::Instant;

#[cfg(test)]
use sn_fake_clock::FakeClock as Instant;

use crate::core::metrics::Metric;
use crate::core::process::Pid;

/// Snapshot of all collected metrics of a single process, from a single probe
///
/// Refer to the [`MetricCollector`](crate::core::collection::MetricCollector) trait to instanciate a `MetricView`
pub struct MetricView<'a> {
    last_metric_date: Instant,
    metrics: Vec<&'a dyn Metric>,
    resolution: Duration,
    default: &'a dyn Metric,
}

impl<'a> MetricView<'a> {
    pub(crate) fn new(
        metrics: Vec<&'a dyn Metric>,
        last_metric_date: Instant,
        resolution: Duration,
        default: &'a dyn Metric,
    ) -> Self {
        Self {
            metrics,
            last_metric_date,
            resolution,
            default,
        }
    }

    /// Returns a slice of the metrics contained in this view.
    /// The slice only covers the last metrics covered by the `span` parameter.
    ///
    /// For example, if the view has a resolution of 1 second, `extract(Duration::from_secs(10))`
    /// will return the last 10 metrics.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago the metrics should be compared
    pub fn extract(&'a self, span: Duration) -> &[&'a dyn Metric] {
        let collected_metrics_count = self.metrics.len();
        let expected_metrics_count = self.calculate_number_of_expected_metrics(span);
        let skipped_metrics = collected_metrics_count.saturating_sub(expected_metrics_count);

        &self.metrics[skipped_metrics..]
    }

    /// Indicates how many metrics should be returned by extract() with the given span, according
    /// to this view's resolution.
    /// Note that the value returned by this function is only an upper bound.
    /// `self.extract()` may return less metrics.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago should metrics be returned
    fn calculate_number_of_expected_metrics(&self, span: Duration) -> usize {
        (span.as_secs() / self.resolution.as_secs()) as usize
    }

    /// Returns the unit representation of the metrics contained in this view
    pub fn unit(&self) -> &'static str {
        self.default.unit()
    }

    /// Indicates the `Duration` step between each metric
    pub fn resolution(&self) -> Duration {
        self.resolution
    }

    /// Returns the latest collected metric, or its default value if no metric has
    /// been collected for this process.
    pub fn last_or_default(&self) -> &dyn Metric {
        *(self.metrics.last().unwrap_or(&self.default))
    }

    /// Returns the greatest f64 value of the metric in the given span. See [`MetricView::new()`](#method.extract) for
    /// the behavior of `span`.
    ///
    /// If the metrics have a cardinality greater than one, the max f64 component of the metric is
    /// used for the comparison.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago the metrics should be compared
    pub fn max_f64(&self, span: Duration) -> f64 {
        self.max_metric(span).max_value()
    }

    /// Returns a concise representation of the greatest metric in the given span. See [`MetricView::new()`](#method.extract) for
    /// the behavior of `span`.
    ///
    /// If the metrics have a cardinality greater than one, the max f64 component of the metric is
    /// used for the comparison.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago the metrics should be compared
    pub fn max_concise_repr(&self, span: Duration) -> String {
        self.max_metric(span).concise_repr()
    }

    fn max_metric(&self, span: Duration) -> &dyn Metric {
        *(self
            .extract(span)
            .iter()
            .max_by(|m1, m2| {
                let v1 = m1.max_value();
                let v2 = m2.max_value();

                v1.partial_cmp(&v2).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(&self.default))
    }

    /// Indicates from when dates the last metric in this view
    pub fn last_metric_date(&self) -> Instant {
        // TODO in core/spv/ui, keep track of time using iteration counter instead of actual Instant instances
        //   only pulse.rs should keep track of time.
        self.last_metric_date
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
mod test_metric_view {
    use sn_fake_clock::FakeClock;
    use std::time::Duration;

    use crate::core::collection::MetricCollection;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::process::ProcessMetadata;
    use crate::core::view::test_helpers::produce_metrics_collection;
    use crate::core::view::MetricView;

    fn build_view_of_pid_0(collection: &MetricCollection<PercentMetric>) -> MetricView {
        collection.view(ProcessMetadata::new(0, "cmd"), Duration::from_secs(1))
    }

    #[test]
    fn test_last_or_default_should_be_latest_metric_when_exists() {
        let collection = produce_metrics_collection(1, vec![0., 1.]);
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.last_or_default(), &PercentMetric::new(1.));
    }

    #[test]
    fn test_last_or_default_should_be_default_when_pid_unknown() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.last_or_default(), &PercentMetric::default());
    }

    #[test]
    fn test_unit_should_be_metric_unit() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.unit(), PercentMetric::default().unit());
    }

    #[test]
    fn test_resolution_should_return_resolution_given_in_constructor() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(ProcessMetadata::new(1, "cmd"), Duration::from_secs(123));

        assert_eq!(view.resolution(), Duration::from_secs(123));
    }

    #[test]
    fn test_extract_should_extract_nothing_if_collection_is_empty() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.extract(Duration::from_secs(60)), &[]);
    }

    #[test]
    fn test_extract_should_return_only_last_metric_if_span_equals_duration() {
        let collection = produce_metrics_collection(1, vec![0., 1.]);
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.extract(view.resolution()), &[view.last_or_default()]);
    }

    #[test]
    fn test_extract_should_return_two_last_metric_if_span_is_double_duration() {
        let collection = produce_metrics_collection(1, vec![0., 1., 2., 3.]);
        let view = build_view_of_pid_0(&collection);

        let extract = view.extract(view.resolution() * 2);

        let expected: &[&dyn Metric; 2] = &[&PercentMetric::new(2.), &PercentMetric::new(3.)];

        assert_eq!(extract, expected);
    }

    #[test]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count() {
        let collection = produce_metrics_collection(1, vec![0., 1., 2.]);
        let view = build_view_of_pid_0(&collection);

        let extract = view.extract(view.resolution() * 20);

        assert_eq!(extract.len(), 3);
        assert_eq!(extract[0], &PercentMetric::new(0.));
        assert_eq!(extract[2], &PercentMetric::new(2.));
    }

    #[test]
    fn test_max_f64_should_return_max_value() {
        let collection = produce_metrics_collection(1, vec![10., 0., 2.]);
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.max_f64(Duration::from_secs(3)), 10.);
    }

    #[test]
    fn test_max_f64_should_not_return_values_out_of_span() {
        let collection = produce_metrics_collection(1, vec![10., 0., 2.]);
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.max_f64(Duration::from_secs(2)), 2.);
    }

    #[test]
    fn test_max_f64_should_return_0_when_empty() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.max_f64(Duration::from_secs(2)), 0.);
    }

    #[test]
    fn test_max_repr_should_return_repr_of_max_value() {
        let collection = produce_metrics_collection(1, vec![0., 10., 2.]);
        let view = build_view_of_pid_0(&collection);

        assert_eq!(view.max_concise_repr(Duration::from_secs(3)), "10.0".to_string());
    }

    #[test]
    fn test_should_return_last_metric_date() {
        let now = FakeClock::now();
        let default = PercentMetric::new(0.);
        let mv = MetricView::new(vec![], now, Duration::from_secs(1), &default);
        assert_eq!(mv.last_metric_date(), now);
    }
}

#[cfg(test)]
mod test_metric_overview {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::view::test_helpers::produce_metrics_collection;
    use crate::core::view::MetricsOverview;

    fn build_overview(collection: &MetricCollection<PercentMetric>) -> MetricsOverview {
        collection.overview()
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

#[cfg(test)]
mod test_helpers {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::PercentMetric;
    use crate::core::process::Pid;

    /// Return collection of PercentMetric containing metrics for `proc_count` processes.<br/>
    /// The Pid values range from `0` to `proc_count - 1`<br/>
    /// To each processes are associated the same PercentMetric values, ranging from `0` to `metrics_count`
    pub(crate) fn produce_metrics_collection(proc_count: usize, values: Vec<f64>) -> MetricCollection<PercentMetric> {
        let mut collection = MetricCollection::new();

        for value in values.into_iter() {
            for proc_idx in 0..proc_count {
                collection.push(proc_idx as Pid, PercentMetric::new(value));
            }
        }

        collection
    }
}
