use std::fmt::{Display, Formatter};
use std::fmt;

/// Errors related to metrics
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    InvalidPercentValue(u8)
}

type Result<T> = std::result::Result<T, Error>;

/// A metric can be a value of any type, as long as two values of the same type can be sorted
pub trait Value: Display + Ord {
    type ValueType: Ord;

    fn value(&self) -> Self::ValueType;
}

/// Metric that has a value between 0 and 100
#[derive(Eq, Ord, PartialOrd, PartialEq, Debug)]
struct PercentValue {
    percent: u8
}

impl PercentValue {
    /// Returns a `PercentMetric`
    /// # Arguments
    ///  * `percent`: A percentage that must be between 0 and 100
    pub fn new(percent: u8) -> Result<PercentValue> {
        if percent > 100 {
            Err(Error::InvalidPercentValue(percent))
        } else {
            Ok(PercentValue { percent })
        }
    }
}

impl Display for PercentValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.percent)
    }
}


impl Value for PercentValue {
    type ValueType = u8;

    fn value(&self) -> Self::ValueType {
        self.percent
    }
}

#[cfg(test)]
mod test_percent_value {
    use super::*;

    #[test]
    fn test_percent_metric_value() {
        let percent_value = PercentValue::new(60)
            .expect("Unexpected error when building PercentValue");

        assert_eq!(percent_value.value(), 60);
    }

    #[test]
    fn test_create_invalid_percent_value() {
        assert_eq!(PercentValue::new(150), Err(Error::InvalidPercentValue(150)));
    }

    #[test]
    fn test_percent_value_cmp() {
        let lesser_val = PercentValue::new(10)
            .expect("Should be valid percent value");
        let greater_val = PercentValue::new(60)
            .expect("Should be valid percent value");

        assert!(lesser_val < greater_val);
        assert!(greater_val > lesser_val);
    }

    #[test]
    fn test_percent_value_fmt() {
        let pv = PercentValue::new(55)
            .expect("Should be a valid percent value");

        assert_eq!(format!("{}", pv), "55%");
    }
}


/// Metric that has a value in bits / seconds
#[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
struct BitrateValue {
    bitrate: u32
}

impl Display for BitrateValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} b/s", self.bitrate)
    }
}


impl BitrateValue {
    /// Returns a `BitrateMetric`
    /// # Arguments
    ///  * `bitrate` A positive value indicating a bitrate in bits/second
    pub fn new(bitrate: u32) -> BitrateValue {
        BitrateValue { bitrate }
    }
}

impl Value for BitrateValue {
    type ValueType = u32;

    fn value(&self) -> Self::ValueType {
        self.bitrate
    }
}

#[cfg(test)]
mod test_bitrate_value {
    use super::*;

    #[test]
    fn test_bitrate_metric_value() {
        let bitrate_value = BitrateValue::new(294830958);

        assert_eq!(bitrate_value.value(), 294830958);
    }

    #[test]
    fn test_bitrate_value_cmp() {
        let lesser_val = BitrateValue::new(123456789);
        let greater_val = BitrateValue::new(987654321);

        assert!(lesser_val < greater_val);
        assert!(greater_val > lesser_val);
    }

    #[test]
    fn test_bitrate_value_fmt() {
        let brv = BitrateValue::new(123);

        assert_eq!(format!("{}", brv), "123 b/s");
    }
}