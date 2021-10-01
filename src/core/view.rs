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

    /// Returns the expected step between each metric
    pub fn step(&self) -> Duration {
        self.resolution
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

    pub fn as_slice(&self) -> &[&dyn Metric] {
        &self.metrics
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
