use std::cmp::Ordering;
use std::fmt::Debug;

use crate::core::Error;

pub trait Metric: Debug {
    fn cardinality(&self) -> usize;
    fn as_f64(&self, index: usize) -> Result<f64, Error>;
    fn max_value(&self) -> f64;

    fn unit(&self) -> &'static str;
    fn concise_repr(&self) -> String;
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


pub trait ClonableMetric: Metric + Clone {}

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
    fn default() -> Self { PercentMetric::new(0.) }
}

impl Metric for PercentMetric {
    fn cardinality(&self) -> usize { 1 }

    fn as_f64(&self, index: usize) -> Result<f64, Error> {
        match index {
            0 => Ok(self.percent_usage),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality()))
        }
    }

    fn max_value(&self) -> f64 { self.percent_usage }

    fn unit(&self) -> &'static str { "%" }

    fn concise_repr(&self) -> String { format!("{:.1}", self.percent_usage) }

    fn explicit_repr(&self, index: usize) -> Result<String, Error> {
        match index {
            0 => Ok(format!("Usage {:.2}%", self.percent_usage)),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality()))
        }
    }
}

impl PartialOrd for PercentMetric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.percent_usage.partial_cmp(&other.percent_usage)
    }
}

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
    fn default() -> Self { IOMetric::new(0, 0) }
}

impl Metric for IOMetric {
    fn cardinality(&self) -> usize { 1 }

    fn as_f64(&self, index: usize) -> Result<f64, Error> {
        match index {
            0 => Ok(self.input as f64),
            1 => Ok(self.output as f64),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality()))
        }
    }

    fn max_value(&self) -> f64 { self.input.max(self.output) as f64 }

    fn unit(&self) -> &'static str { "B/s" }

    fn concise_repr(&self) -> String { formatted_bytes(self.max_value() as usize, 1) }

    fn explicit_repr(&self, index: usize) -> Result<String, Error> {
        match index {
            0 => Ok(format!("{}B/s", formatted_bytes(self.input, 2))),
            1 => Ok(format!("{}B/s", formatted_bytes(self.output, 2))),
            _ => Err(Error::RawMetricAccessError(index, self.cardinality()))
        }
    }
}

/// Returns a more readable version of `bytes_val`
/// `formatted_bytes(1294221)` -> 1.2M
fn formatted_bytes(bytes_val: usize, precision: usize) -> String {
    if bytes_val == 0 {
        return "0".to_string();
    }

    const METRIC_PREFIXES: [&str; 4] = ["", "k", "M", "G"];

    let log = (bytes_val as f64).log(1024.)
        .max(0.).floor() as usize;

    let prefix_index = log.min(METRIC_PREFIXES.len() - 1);

    let simplified = bytes_val as f64 / (1024_usize.pow(log as u32) as f64);

    format!("{:.precision$}{}", simplified, METRIC_PREFIXES[prefix_index], precision = precision)
}


impl PartialOrd for IOMetric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.max_value().partial_cmp(&other.max_value())
    }
}