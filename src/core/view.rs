use std::collections::HashMap;
use std::time::Duration;

use crate::core::metrics::Metric;
use crate::core::process_view::Pid;

pub struct MetricView<'a> {
    metrics: Vec<&'a dyn Metric>,
    resolution: Duration,
    default: &'a dyn Metric,
}

impl<'a> MetricView<'a> {
    pub fn new(metrics: Vec<&'a dyn Metric>, resolution: Duration, default: &'a dyn Metric) -> Self {
        Self { metrics, resolution, default }
    }

    pub fn extract(&'a self, span: Duration) -> &[&'a dyn Metric] {
        let collected_metrics_count = self.metrics.len();
        let expected_metrics_count = self.expected_metrics(span);
        let skipped_metrics = collected_metrics_count.saturating_sub(expected_metrics_count);

        &self.metrics[skipped_metrics..]
    }

    /// Indicates how many metrics should be returned by history() with the given span, according
    /// to this archive's resolution.
    /// Note that The value returned by this function is not a guarantee. History() may return less.
    ///
    /// # Arguments
    ///  * span: Indicates from how long ago should metrics be returned
    fn expected_metrics(&self, span: Duration) -> usize {
        (span.as_secs() / self.resolution.as_secs()) as usize
    }

    pub fn unit(&self) -> &'static str {
        self.default.unit()
    }

    pub fn resolution(&self) -> Duration { self.resolution }

    pub fn last_or_default(&self) -> &dyn Metric {
        *(self.metrics.last()
            .unwrap_or(&self.default))
    }
}

pub struct MetricsOverview<'a> {
    last_metrics: HashMap<Pid, &'a dyn Metric>,
    default: &'a dyn Metric,
}

impl<'a> MetricsOverview<'a> {
    pub fn new(last_metrics: HashMap<Pid, &'a dyn Metric>, default: &'a dyn Metric) -> Self {
        Self { last_metrics, default }
    }

    pub fn last_or_default(&self, pid: Pid) -> &dyn Metric {
        *(self.last_metrics.get(&pid)
            .unwrap_or(&self.default))
    }

    pub fn unit(&self) -> &'static str {
        self.default.unit()
    }
}


#[cfg(test)]
mod test_metric_view {
    use std::time::Duration;

    use crate::core::collection::MetricCollection;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::view::MetricView;
    use crate::core::view::test_helpers::produce_metrics_collection;

    fn build_view(collection: &MetricCollection<PercentMetric>) -> MetricView {
        collection.view(0, Duration::from_secs(1))
    }

    #[test]
    fn test_last_or_default_should_be_latest_metric_when_exists() {
        let collection = produce_metrics_collection(1, 2);
        let view = build_view(&collection);

        assert_eq!(view.last_or_default(), &PercentMetric::new(1.));
    }

    #[test]
    fn test_last_or_default_should_be_default_when_pid_unknown() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view(&collection);

        assert_eq!(view.last_or_default(), &PercentMetric::default());
    }

    #[test]
    fn test_unit_should_be_metric_unit() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view(&collection);

        assert_eq!(view.unit(), PercentMetric::default().unit());
    }

    #[test]
    fn test_resolution_should_return_resolution_given_in_constructor() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = collection.view(1, Duration::from_secs(123));

        assert_eq!(view.resolution(), Duration::from_secs(123));
    }

    #[test]
    fn test_extract_should_extract_nothing_if_collection_is_empty() {
        let collection = MetricCollection::<PercentMetric>::new();
        let view = build_view(&collection);

        assert_eq!(view.extract(Duration::from_secs(60)), &[]);
    }

    #[test]
    fn test_extract_should_return_only_last_metric_if_span_equals_duration() {
        let collection = produce_metrics_collection(1, 10);
        let view = build_view(&collection);

        assert_eq!(view.extract(view.resolution()), &[view.last_or_default()]);
    }

    #[test]
    fn test_extract_should_return_two_last_metric_if_span_is_double_duration() {
        let collection = produce_metrics_collection(1, 10);
        let view = build_view(&collection);

        let extract = view.extract(view.resolution() * 2);

        let expected: &[&dyn Metric; 2] = &[
            &PercentMetric::new(8.),
            &PercentMetric::new(9.)
        ];

        assert_eq!(extract, expected);
    }

    #[test]
    fn test_should_only_return_existing_items_when_span_greater_than_metric_count() {
        let collection = produce_metrics_collection(1, 10);
        let view = build_view(&collection);

        let extract = view.extract(view.resolution() * 20);

        assert_eq!(extract.len(), 10);
        assert_eq!(extract[0], &PercentMetric::new(0.));
        assert_eq!(extract[9], &PercentMetric::new(9.));
    }
}


#[cfg(test)]
mod test_metric_overview {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::{Metric, PercentMetric};
    use crate::core::view::MetricsOverview;
    use crate::core::view::test_helpers::produce_metrics_collection;

    fn build_overview(collection: &MetricCollection<PercentMetric>) -> MetricsOverview {
        collection.overview()
    }

    fn test_unit_should_be_default_metric_unit() {
        let collection = produce_metrics_collection(2, 2);
        let overview = build_overview(&collection);

        assert_eq!(overview.unit(), PercentMetric::default().unit());
    }

    fn test_last_or_default_should_return_last_when_proc_has_metrics() {
        let collection = produce_metrics_collection(2, 2);
        let overview = build_overview(&collection);

        assert_eq!(overview.last_or_default(0), &PercentMetric::new(1.));
    }

    fn test_last_or_default_should_return_default_when_pid_is_unknown() {
        let collection = produce_metrics_collection(2, 2);
        let overview = build_overview(&collection);

        assert_eq!(overview.last_or_default(2), &PercentMetric::default());
    }
}

#[cfg(test)]
mod test_helpers {
    use crate::core::collection::MetricCollection;
    use crate::core::metrics::PercentMetric;
    use crate::core::process_view::Pid;

    /// Return collection of PercentMetric containing metrics for `proc_count` processes.<br/>
    /// The Pid values range from `0` to `proc_count - 1`<br/>
    /// To each processes are associated the same PercentMetric values, ranging from `0` to `metrics_count`
    pub(crate) fn produce_metrics_collection(proc_count: usize, metrics_count: usize) -> MetricCollection<PercentMetric> {
        let mut collection = MetricCollection::new();

        for proc_idx in 0..proc_count {
            for metric_idx in 0..metrics_count {
                collection.push(proc_idx as Pid, PercentMetric::new(metric_idx as f64));
            }
        }

        collection
    }
}