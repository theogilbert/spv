//! Definition of the different types of metrics

use std::cmp::Ordering;
use std::fmt::Debug;

use crate::core::time::Timestamp;
use crate::core::Error;

/// Types which represent a measurement of some kind.
///
/// This trait allows applications to handle any type of `Metric` generically.
pub trait Metric: Debug {
    /// Indicates how many components this metric has
    ///
    /// Instances of a given concrete `Metric` type must always return the same cardinality
    fn cardinality(&self) -> usize;

    /// Extracts a component of the metric as a float number
    ///
    /// If `index` is greater than or equal to `cardinality()`, an error is returned instead
    ///
    /// # Arguments
    ///  * index: Indicates the component to extract
    ///
    fn as_f64(&self, index: usize) -> Result<f64, Error>;

    /// Returns the highest component of the metric, as a float
    fn max_value(&self) -> f64;

    /// Returns the unit representation of the metric
    fn unit(&self) -> &'static str;

    /// Returns a concise representation of the metric
    ///
    /// This representation may not include all components of the metric
    fn concise_repr(&self) -> String;

    /// For a given value to be interpreted as the value returned by [`max_value()`](#method.max_value), return a
    /// concise representation of it.
    ///
    /// # Arguments
    /// * `value`: The value for which to generate a concise representation
    fn concise_repr_of_value(&self, value: f64) -> String;

    /// Returns an explicit representation of a component of the metric
    ///
    /// # Arguments
    ///   * index: Indicates the component of which to get a representation
    fn explicit_repr(&self, index: usize) -> Result<String, Error>;
}

#[cfg(test)]
impl PartialEq for &dyn Metric {
    // Helper PartialEq impl to make tests more readable
    fn eq(&self, other: &Self) -> bool {
        if self.cardinality() != other.cardinality() {
            return false;
        } else if self.unit() != other.unit() {
            return false;
        }

        for i in 0..self.cardinality() {
            if self.as_f64(i).unwrap() != other.as_f64(i).unwrap() {
                return false;
            }
        }

        true
    }
}

/// Bundles a reference to a metric, and its timestamp
pub struct DatedMetric<'a> {
    /// The timestamp at which the metric was collected
    pub timestamp: Timestamp,
    /// A reference to a metric trait object
    pub metric: &'a dyn Metric,
}

impl<'a> DatedMetric<'a> {
    pub fn new(metric: &'a dyn Metric, timestamp: Timestamp) -> Self {
        Self { metric, timestamp }
    }
}

/// Metric representing a percent value (e.g. CPU usage)
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct PercentMetric {
    percent_usage: f64,
}

impl PercentMetric {
    pub fn new(percent_usage: f64) -> Self {
        Self { percent_usage }
    }
}

impl Default for PercentMetric {
    fn default() -> Self {
        PercentMetric::new(0.)
    }
}

impl Metric for PercentMetric {
    /// Returns 1, as PercentMetric is only composed of one element: the percent value.
    fn cardinality(&self) -> usize {
        1
    }

    fn as_f64(&self, index: usize) -> Result<f64, Error> {
        match index {
            0 => Ok(self.percent_usage),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality())),
        }
    }

    fn max_value(&self) -> f64 {
        self.percent_usage
    }

    fn unit(&self) -> &'static str {
        "%"
    }

    fn concise_repr(&self) -> String {
        self.concise_repr_of_value(self.percent_usage)
    }

    fn concise_repr_of_value(&self, value: f64) -> String {
        format!("{:.1}", value)
    }

    fn explicit_repr(&self, index: usize) -> Result<String, Error> {
        match index {
            0 => Ok(format!("Usage {:.2}%", self.percent_usage)),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality())),
        }
    }
}

impl PartialOrd for PercentMetric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.percent_usage.partial_cmp(&other.percent_usage)
    }
}

#[cfg(test)]
mod test_percent_metric {
    use std::cmp::Ordering;

    use crate::core::metrics::{Metric, PercentMetric};

    #[test]
    fn test_should_return_sole_value_as_max_value() {
        let metric = PercentMetric::new(10.);
        assert_eq!(metric.max_value(), 10.);
    }

    #[test]
    fn test_should_correctly_compare_metrics_based_on_percent_value() {
        let lesser_metric = PercentMetric::new(10.);
        let greater_metric = PercentMetric::new(20.);

        assert_eq!(lesser_metric.partial_cmp(&greater_metric), Some(Ordering::Less));
        assert_eq!(greater_metric.partial_cmp(&lesser_metric), Some(Ordering::Greater));
    }
}

/// Metric representing input / output bitrates (e.g. network throughput) in bytes/sec
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IOMetric {
    input: usize,
    output: usize,
}

impl IOMetric {
    pub fn new(input: usize, output: usize) -> Self {
        Self { input, output }
    }
}

impl Default for IOMetric {
    fn default() -> Self {
        IOMetric::new(0, 0)
    }
}

impl Metric for IOMetric {
    /// Returns 2, as a IOMetric is composed of two elements: the input and the output values
    fn cardinality(&self) -> usize {
        2
    }

    fn as_f64(&self, index: usize) -> Result<f64, Error> {
        match index {
            0 => Ok(self.input as f64),
            1 => Ok(self.output as f64),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality())),
        }
    }

    fn max_value(&self) -> f64 {
        self.input.max(self.output) as f64
    }

    fn unit(&self) -> &'static str {
        "B/s"
    }

    fn concise_repr(&self) -> String {
        self.concise_repr_of_value(self.max_value())
    }

    fn concise_repr_of_value(&self, value: f64) -> String {
        format_bytes(value as usize, 1)
    }

    fn explicit_repr(&self, index: usize) -> Result<String, Error> {
        match index {
            0 => Ok(format!("Input : {}B/s", format_bytes(self.input, 2))),
            1 => Ok(format!("Output: {}B/s", format_bytes(self.output, 2))),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality())),
        }
    }
}

/// Returns a user-friendly representation of `bytes_val`
///
/// # Examples:
///
/// ```ignore
/// assert_eq!(formatted_bytes(123), "123".to_string());
/// assert_eq!(formatted_bytes(1294221), "1.2M".to_string());
/// ```
fn format_bytes(bytes_val: usize, precision: usize) -> String {
    if bytes_val == 0 {
        return "0".to_string();
    }

    const METRIC_PREFIXES: [&str; 4] = ["", "k", "M", "G"];

    let prefix_index = (bytes_val as f64)
        .log(1024.)
        .max(0.)
        .min((METRIC_PREFIXES.len() - 1) as f64)
        .floor() as usize;

    let simplified = bytes_val as f64 / (1024_usize.pow(prefix_index as u32) as f64);

    format!(
        "{:.precision$}{}",
        simplified,
        METRIC_PREFIXES[prefix_index],
        precision = precision
    )
}

impl PartialOrd for IOMetric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.max_value().partial_cmp(&other.max_value())
    }
}

#[cfg(test)]
mod test_io_metric {
    use std::cmp::Ordering;

    use crate::core::metrics::{IOMetric, Metric};

    #[test]
    fn test_max_value_should_return_input_as_float_when_input_greater() {
        let metric = IOMetric::new(10, 5);
        assert_eq!(metric.max_value(), 10.);
    }

    #[test]
    fn test_max_value_should_return_output_as_float_when_output_greater() {
        let metric = IOMetric::new(10, 20);
        assert_eq!(metric.max_value(), 20.);
    }

    #[test]
    fn test_first_component_should_be_input() {
        let metric = IOMetric::new(1, 0);
        assert_eq!(metric.as_f64(0).unwrap(), 1.);
    }

    #[test]
    fn test_second_component_should_be_output() {
        let metric = IOMetric::new(1, 0);
        assert_eq!(metric.as_f64(1).unwrap(), 0.);
    }

    #[test]
    fn test_should_order_io_metrics_based_on_max_value() {
        let lesser_metric = IOMetric::new(10, 20);
        let greater_metric = IOMetric::new(15, 25);

        assert_eq!(lesser_metric.partial_cmp(&greater_metric), Some(Ordering::Less));
        assert_eq!(greater_metric.partial_cmp(&lesser_metric), Some(Ordering::Greater));
    }
}

#[cfg(test)]
mod test_formatted_bytes {
    use rstest::*;

    use crate::core::metrics::format_bytes;

    #[rstest]
    #[case(42, "42.00")]
    #[case(2048, "2.00k")]
    #[case(3000, "2.93k")]
    #[case(1024 * 1024, "1.00M")]
    #[case(1500000, "1.43M")]
    #[case(1024 * 1024 * 1024, "1.00G")]
    #[case(1500000000, "1.40G")]
    #[case(1024 * 1024 * 1024 * 1024, "1024.00G")]
    fn test_should_reformat_bytes_correctly(#[case] input: usize, #[case] expected: &str) {
        let fmted = format_bytes(input, 2);
        assert_eq!(fmted, expected.to_string());
    }
}
