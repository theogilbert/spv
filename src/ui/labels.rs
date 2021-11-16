//! Generates human-readable labels from raw data

use std::time::Duration;

use crate::core::iteration::Iteration;
use crate::ui::Error;

type Milliseconds = u128;

/// Generates String labels indicating the relative time at which an iteration occured
pub struct TimeLabelMaker {
    iteration_span: Milliseconds,
}

impl TimeLabelMaker {
    /// Instanciates a new `LabelMaker`
    ///
    /// # Arguments
    /// * `resolution`: Indicates the duration of 1 iteration
    pub fn new(resolution: Duration) -> Self {
        Self {
            iteration_span: resolution.as_millis(),
        }
    }

    /// Generates a label describing the time offset between `current_iter` and `iter_to_label`<br/>
    /// If `iter_to_label` is greater than `current_iter`, this method returns an error.
    ///
    /// # Arguments
    /// * `current_iter`: The current iteration of the application
    /// * `iter_to_label`: The iteration for which to generate a label.<br/> Must be less or equal than `current_iter`.
    pub fn relative_label(&self, current_iter: Iteration, iter_to_label: Iteration) -> Result<String, Error> {
        if iter_to_label > current_iter {
            return Err(Error::InvalidIterationValue(current_iter, iter_to_label));
        }

        let total_delta_in_sec = ((current_iter - iter_to_label) as u128 * self.iteration_span / 1000) as u64;
        Ok(format_time_delta(total_delta_in_sec))
    }
}

fn format_time_delta(delta_in_sec: u64) -> String {
    let hours_component = delta_in_sec / 3600;
    let minutes_component = (delta_in_sec / 60) % 60;
    let seconds_component = delta_in_sec % 60;

    if delta_in_sec == 0 {
        "now".to_string()
    } else if hours_component > 99 {
        format!("{}h ago", hours_component)
    } else if hours_component > 0 {
        format!("{}h {}m ago", hours_component, minutes_component)
    } else if minutes_component > 0 {
        format!("{}m {}s ago", minutes_component, seconds_component)
    } else {
        format!("{}s ago", seconds_component)
    }
}

#[cfg(test)]
mod test_relative_time_label {
    use std::time::Duration;

    use rstest::*;

    use crate::ui::labels::{format_time_delta, TimeLabelMaker};
    use crate::ui::Error;

    #[test]
    fn test_should_return_error_if_iter_greater_than_current_iter() {
        let label_maker = TimeLabelMaker::new(Duration::from_secs(1));
        let error = label_maker.relative_label(10, 11).err().unwrap();

        assert!(matches!(error, Error::InvalidIterationValue(10, 11)));
    }

    #[rstest]
    #[case(0)]
    #[case(200)]
    #[case(999)]
    fn test_should_be_now_when_delta_is_under_1_sec(#[case] delta_in_ms: u64) {
        let label_maker = TimeLabelMaker::new(Duration::from_millis(delta_in_ms));
        let label = label_maker.relative_label(10, 9).unwrap();

        assert_eq!(label, "now".to_string());
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    fn test_should_be_n_sec_ago_when_delta_is_n_sec(#[case] n: usize) {
        let label_maker = TimeLabelMaker::new(Duration::from_secs(1));
        let label = label_maker.relative_label(10, 10 - n).unwrap();

        assert_eq!(label, format!("{}s ago", n));
    }

    #[rstest]
    #[case(500, 4, "2s ago")]
    #[case(2000, 4, "8s ago")]
    fn test_should_reflect_resolution(
        #[case] resolution_in_ms: u64,
        #[case] delta: usize,
        #[case] expected_result: String,
    ) {
        let label_maker = TimeLabelMaker::new(Duration::from_millis(resolution_in_ms));
        let label = label_maker.relative_label(100, 100 - delta).unwrap();

        assert_eq!(label, expected_result);
    }

    #[rstest]
    #[case(60, "1m 0s ago")]
    #[case(75, "1m 15s ago")]
    #[case(3600, "1h 0m ago")]
    #[case(3720, "1h 2m ago")]
    #[case(3725, "1h 2m ago")]
    fn test_should_include_two_components_at_most(#[case] delta: u64, #[case] expected: String) {
        let label = format_time_delta(delta);
        assert_eq!(label, expected);
    }

    #[test]
    fn test_should_display_hour_and_minute_when_under_100_hours() {
        let label = format_time_delta(99 * 60 * 60 + 300);
        assert_eq!(label, "99h 5m ago");
    }

    #[test]
    fn test_should_display_only_hour_when_greater_than_99_hours() {}

    #[test]
    fn test_should_not_display_unit_greater_than_hour() {
        let label = format_time_delta(100 * 60 * 60 + 300);
        assert_eq!(label, "100h ago");
    }
}
