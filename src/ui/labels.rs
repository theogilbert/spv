//! Generates human-readable labels from raw data

use crate::core::time::Timestamp;

/// Generates a label describing the time offset between `current_iter` and `iter_to_label`<br/>
/// If `iter_to_label` is greater than `current_iter`, this method returns an error.
///
/// # Arguments
/// * `timestamp`: The timestmap for which to generate a label
pub fn relative_timestamp_label(timestamp: Timestamp) -> String {
    let delta_in_sec = Timestamp::now().duration_since(&timestamp).as_secs();

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

    use crate::core::time::test_utils::setup_fake_clock_to_prevent_substract_overflow;
    use crate::core::time::Timestamp;
    use rstest::*;

    use crate::ui::labels::relative_timestamp_label;

    #[rstest]
    #[case(0)]
    #[case(200)]
    #[case(999)]
    fn test_should_be_now_when_delta_is_under_1_sec(#[case] delta_in_ms: u64) {
        setup_fake_clock_to_prevent_substract_overflow();

        let timestamp = Timestamp::now() - Duration::from_millis(delta_in_ms);
        let label = relative_timestamp_label(timestamp);

        assert_eq!(label, "now".to_string());
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(10)]
    fn test_should_be_n_sec_ago_when_delta_is_n_sec(#[case] n: u64) {
        setup_fake_clock_to_prevent_substract_overflow();

        let timestamp = Timestamp::now() - Duration::from_secs(n);
        let label = relative_timestamp_label(timestamp);

        assert_eq!(label, format!("{}s ago", n));
    }

    #[rstest]
    #[case(60, "1m 0s ago")]
    #[case(75, "1m 15s ago")]
    #[case(3600, "1h 0m ago")]
    #[case(3720, "1h 2m ago")]
    #[case(3725, "1h 2m ago")]
    fn test_should_include_two_components_at_most(#[case] delta_in_secs: u64, #[case] expected: String) {
        setup_fake_clock_to_prevent_substract_overflow();

        let timestamp = Timestamp::now() - Duration::from_secs(delta_in_secs);
        let label = relative_timestamp_label(timestamp);

        assert_eq!(label, expected);
    }

    #[test]
    fn test_should_display_hour_and_minute_when_under_100_hours() {
        setup_fake_clock_to_prevent_substract_overflow();

        let timestamp = Timestamp::now() - Duration::from_secs(99 * 60 * 60 + 300);
        let label = relative_timestamp_label(timestamp);

        assert_eq!(label, "99h 5m ago");
    }

    #[test]
    fn test_should_display_only_hour_when_greater_than_99_hours() {
        setup_fake_clock_to_prevent_substract_overflow();

        let timestamp = Timestamp::now() - Duration::from_secs(100 * 60 * 60 + 300);
        let label = relative_timestamp_label(timestamp);

        assert_eq!(label, "100h ago");
    }
}
