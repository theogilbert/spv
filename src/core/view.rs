//! View of collected metrics

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::core::iteration::{IterSpan, Iteration};
use crate::core::metrics::Metric;
use crate::core::process::Pid;

/// Snapshot of all collected metrics of a single process, from a single probe
///
/// Refer to the [`MetricCollector`](crate::core::collection::MetricCollector) trait to instanciate a `MetricView`
pub struct MetricView<'a> {
    metrics: Vec<&'a dyn Metric>,
    default: &'a dyn Metric,
    span: IterSpan,
    last_metric_iteration: Iteration,
    current_iteration: Iteration,
}

impl<'a> MetricView<'a> {
    pub(crate) fn new(
        metrics: Vec<&'a dyn Metric>,
        default: &'a dyn Metric,
        span: IterSpan,
        last_metric_iteration: Iteration,
        current_iteration: Iteration,
    ) -> Self {
        Self {
            metrics,
            default,
            span,
            last_metric_iteration,
            current_iteration,
        }
    }

    /// Returns a slice of the metrics contained in this view.
    /// The slice only covers the last metrics covered by the `span` parameter.
    pub fn as_slice(&'a self) -> &[&'a dyn Metric] {
        &self.metrics
    }

    /// Returns the unit representation of the metrics contained in this view
    pub fn unit(&self) -> &'static str {
        self.default.unit()
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
    pub fn max_concise_repr(&self) -> String {
        self.max_metric().concise_repr()
    }

    fn max_metric(&self) -> &dyn Metric {
        *(self
            .metrics
            .iter()
            .max_by(|m1, m2| {
                let v1 = m1.max_value();
                let v2 = m2.max_value();

                v1.partial_cmp(&v2).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(&self.default))
    }

    pub fn span(&self) -> usize {
        self.span.span()
    }

    /// Indicates from when dates the last metric in this view
    pub fn last_iteration(&self) -> Iteration {
        self.last_metric_iteration
    }

    pub fn current_iteration(&self) -> Iteration {
        self.current_iteration
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
    use crate::core::collection::MetricCollection;
    use crate::core::iteration::IterSpan;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::view::test_helpers::produce_metrics_collection;

    #[test]
    fn test_last_or_default_should_be_latest_metric_when_exists() {
        let collection = produce_metrics_collection(1, vec![0., 1.]);
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.last_or_default(), &PercentMetric::new(1.));
    }

    #[test]
    fn test_last_or_default_should_be_default_when_pid_unknown() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.last_or_default(), &PercentMetric::default());
    }

    #[test]
    fn test_unit_should_be_metric_unit() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.unit(), PercentMetric::default().unit());
    }

    #[test]
    fn test_extract_should_extract_nothing_if_collection_is_empty() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.as_slice(), &[]);
    }

    #[test]
    fn test_extract_should_return_only_last_metric_if_span_coveres_1_iteration() {
        let collection = produce_metrics_collection(1, vec![0., 1.]);
        let view = collection.view(0, IterSpan::new(1), 10);

        assert_eq!(view.as_slice(), &[view.last_or_default()]);
    }

    #[test]
    fn test_extract_should_return_two_last_metric_if_span_covers_2_iterations() {
        let collection = produce_metrics_collection(1, vec![0., 1., 2., 3.]);
        let view = collection.view(0, IterSpan::new(2), 10);

        let expected: &[&dyn Metric; 2] = &[&PercentMetric::new(2.), &PercentMetric::new(3.)];

        assert_eq!(view.as_slice(), expected);
    }

    #[test]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count() {
        let collection = produce_metrics_collection(1, vec![0., 1., 2.]);
        let view = collection.view(0, IterSpan::new(20), 10);
        let extract = view.as_slice();

        assert_eq!(extract.len(), 3);
        assert_eq!(extract[0], &PercentMetric::new(0.));
        assert_eq!(extract[2], &PercentMetric::new(2.));
    }

    #[test]
    fn test_max_f64_should_return_max_value() {
        let collection = produce_metrics_collection(1, vec![10., 0., 2.]);
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.max_f64(), 10.);
    }

    #[test]
    fn test_max_f64_should_not_return_values_out_of_span() {
        let collection = produce_metrics_collection(1, vec![10., 0., 2.]);
        let view = collection.view(0, IterSpan::new(2), 10);

        assert_eq!(view.max_f64(), 2.);
    }

    #[test]
    fn test_max_f64_should_return_0_when_empty() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.max_f64(), 0.);
    }

    #[test]
    fn test_max_repr_should_return_repr_of_max_value() {
        let collection = produce_metrics_collection(1, vec![0., 10., 2.]);
        let view = collection.view(0, IterSpan::default(), 10);

        assert_eq!(view.max_concise_repr(), "10.0".to_string());
    }

    #[test]
    fn test_should_return_0_as_default_last_iteration() {
        let collection = MetricCollection::<PercentMetric>::new();

        let view = collection.view(1, IterSpan::default(), 10);
        assert_eq!(view.last_iteration(), 0);
    }

    #[test]
    fn test_should_return_last_iteration() {
        let mut collection = MetricCollection::new();
        collection.push(1, PercentMetric::new(10.), 1);
        collection.push(1, PercentMetric::new(10.), 2);

        let view = collection.view(1, IterSpan::default(), 10);
        assert_eq!(view.last_iteration(), 2);
    }

    #[test]
    fn test_should_return_correct_span() {
        let mut collection = MetricCollection::new();
        collection.push(1, PercentMetric::new(10.), 1);
        collection.push(1, PercentMetric::new(10.), 2);

        let view = collection.view(1, IterSpan::new(123), 10);
        assert_eq!(view.span(), 123);
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

        for (iteration, value) in values.into_iter().enumerate() {
            for proc_idx in 0..proc_count {
                collection.push(proc_idx as Pid, PercentMetric::new(value), iteration);
            }
        }

        collection
    }
}
