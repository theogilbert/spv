use std::fmt::{Display, Formatter};
use std::fmt;

use crate::core::Error;

type Result<T> = std::result::Result<T, Error>;

/// A metric can be a value of any type, as long as two values of the same type can be sorted
pub trait Value: Display + PartialOrd {
    type ValueType: PartialOrd;

    fn value(&self) -> Self::ValueType;

    fn unit(&self) -> String;
}

/// Metric that has a value between 0 and 100
#[derive(PartialEq, PartialOrd, Debug, Copy, Clone)]
pub struct Percent {
    percent: f32
}

impl Percent {
    /// Returns a `PercentMetric`
    /// # Arguments
    ///  * `percent`: A percentage that must be between 0 and 100
    pub fn new(percent: f32) -> Result<Percent> {
        if percent < 0. || percent > 100. {
            Err(Error::InvalidPercentValue(percent))
        } else {
            Ok(Percent { percent })
        }
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}", self.percent)
    }
}


impl Value for Percent {
    type ValueType = f32;

    fn value(&self) -> Self::ValueType {
        self.percent
    }

    fn unit(&self) -> String {
        "%".to_string()
    }
}

#[cfg(test)]
mod test_percent_value {
    use super::*;

    #[test]
    fn test_percent_metric_value() {
        let percent_value = Percent::new(60.)
            .expect("Unexpected error when building PercentValue");

        assert_eq!(percent_value.value(), 60.);
    }

    #[test]
    fn test_create_too_great_percent_value() {
        assert_eq!(Percent::new(150.), Err(Error::InvalidPercentValue(150.)));
    }

    #[test]
    fn test_create_negative_percent_value() {
        assert_eq!(Percent::new(-1.), Err(Error::InvalidPercentValue(-1.)));
    }

    #[test]
    fn test_percent_value_cmp() {
        let lesser_val = Percent::new(10.)
            .expect("Should be valid percent value");
        let greater_val = Percent::new(60.)
            .expect("Should be valid percent value");

        assert!(lesser_val < greater_val);
        assert!(greater_val > lesser_val);
    }

    #[test]
    fn test_percent_value_fmt() {
        let pv = Percent::new(55.04)
            .expect("Should be a valid percent value");

        assert_eq!(format!("{}", pv), "55.0");
    }
}


/// Metric that has a value in bits / seconds
#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub struct Bitrate {
    bitrate: u32
}


impl Bitrate {
    /// Returns a `BitrateMetric`
    /// # Arguments
    ///  * `bitrate` A positive value indicating a bitrate in bits/second
    pub fn new(bitrate: u32) -> Bitrate {
        Bitrate { bitrate }
    }
}

impl Value for Bitrate {
    type ValueType = u32;

    fn value(&self) -> Self::ValueType {
        self.bitrate
    }

    fn unit(&self) -> String {
        "bps".to_string()  // TODO update unit to adapt depending on value (b/s, Kb/s, Mb/s, ...)
    }
}

impl Display for Bitrate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.bitrate)
    }
}

#[cfg(test)]
mod test_bitrate_value {
    use super::*;

    #[test]
    fn test_bitrate_metric_value() {
        let bitrate_value = Bitrate::new(294830958);

        assert_eq!(bitrate_value.value(), 294830958);
    }

    #[test]
    fn test_bitrate_value_cmp() {
        let lesser_val = Bitrate::new(123456789);
        let greater_val = Bitrate::new(987654321);

        assert!(lesser_val < greater_val);
        assert!(greater_val > lesser_val);
    }

    #[test]
    fn test_bitrate_value_fmt() {
        let brv = Bitrate::new(123);

        assert_eq!(format!("{}", brv), "123");
    }
}