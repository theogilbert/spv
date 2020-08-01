use std::collections::hash_map::RandomState;
use std::collections::HashSet;

use crate::probe::{Error, Probe, ProcessMetric};
use crate::process::PID;
use crate::values::{Bitrate, Percent};

#[derive(PartialEq, Debug)]
pub enum Metrics {
    Percents(Vec<ProcessMetric<Percent>>),
    Bitrates(Vec<ProcessMetric<Bitrate>>),
}

#[derive(PartialEq, Debug)]
pub struct LabelledMetrics {
    label: String,
    metrics: Metrics,
}

pub struct Frame {
    // TODO this should be the struct that normalizes metrics
    labelled_metrics: Vec<LabelledMetrics>,
}

impl<'a> Frame {
    pub fn new(metrics: Vec<LabelledMetrics>) -> Self {
        // TODO if a PID is not in one of the metrics, remove it from all others
        Self { labelled_metrics: metrics }
    }

    pub fn labels(&'a self) -> Vec<&'a str> {
        self.labelled_metrics.iter()
            .map(|lm| lm.label.as_str())
            .collect()
    }

    pub fn metrics(&'a self, label: &str) -> Option<&'a Metrics> {
        self.labelled_metrics.iter()
            .find(|lm| lm.label == label)
            .map(|lm| &lm.metrics)
    }
}

#[cfg(test)]
mod test_frame {
    use crate::probe::dispatch::{Frame, LabelledMetrics, Metrics};
    use crate::probe::ProcessMetric;
    use crate::values::{Bitrate, Percent};

    fn get_example_metrics() -> Vec<LabelledMetrics> {
        vec![
            LabelledMetrics {
                label: "metrics_1".to_string(),
                metrics: Metrics::Percents(vec![ProcessMetric { pid: 123, value: Percent::new(50.).unwrap() }]),
            },
            LabelledMetrics {
                label: "metrics_2".to_string(),
                metrics: Metrics::Bitrates(vec![ProcessMetric { pid: 123, value: Bitrate::new(50) }]),
            },
        ]
    }

    #[test]
    fn test_should_return_correct_labels() {
        assert_eq!(Frame::new(get_example_metrics()).labels(),
                   vec!["metrics_1", "metrics_2"]);
    }

    #[test]
    fn test_should_return_correct_values() {
        assert_eq!(Frame::new(get_example_metrics()).metrics("metrics_1"),
                   Some(&Metrics::Percents(vec![ProcessMetric {
                       pid: 123,
                       value: Percent::new(50.).unwrap(),
                   }])))
    }
}

