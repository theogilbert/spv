//! Manages the configuration of the span of metrics to render
use log::info;
use std::cmp::min;
use std::time::Duration;

use crate::core::time::{Span, Timestamp};

/**
The smallest amount of duration for the rendering span.

A value of 15 was picked to be able to configure the size of the span by dividing/multiplying the current size by 2, and
to be able to display a span of 1 minute by default.
 */
const SPAN_UNIT: Duration = Duration::from_secs(15);

const DEFAULT_SPAN_DURATION: Duration = Duration::from_secs(60);

pub struct RenderingSpan {
    span: Span,
    follow: bool,
    // Span size can be calculated from zoom_level using this formula: 15s * 2^zoom_level
    zoom_level: u32,
}

impl RenderingSpan {
    /// # Arguments
    /// - `duration`: Indicates the amount of time that the span covers
    /// - `tolerance`: Tracking time precisely to the nanosecond is difficult.<br/>
    ///     The tolerance, will loosen the constraints of the span, by shifting its begin to the past.
    pub fn new(duration: Duration) -> Self {
        Self {
            span: Span::from_duration(duration),
            follow: true,
            zoom_level: 2,
        }
    }

    /// Shifts the rendering span so that it ends at the current time
    pub fn follow(&mut self) {
        if self.follow {
            self.span.set_end_and_shift(Timestamp::now());
            self.set_follow_if_span_is_tracking_current_timestamp();
        }
    }
    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the past
    ///
    /// The span cannot be scrolled before the first iteration of the program
    pub fn scroll_left(&mut self) {
        self.set_bounded_end_and_shift(self.span.end() - Duration::from_secs(1));
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the future
    ///
    /// The span cannot be scrolled after the current timestamp.
    pub fn scroll_right(&mut self) {
        self.set_bounded_end_and_shift(self.span.end() + Duration::from_secs(1));
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    /// Reset the span so that it tracks the latest metrics
    pub fn reset_scroll(&mut self) {
        self.span.set_end_and_shift(Timestamp::now());
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    fn set_follow_if_span_is_tracking_current_timestamp(&mut self) {
        self.follow = self.span.end() == Timestamp::now();
    }

    /// Sets the end of the span and shift it (without resizing it)
    /// The end is capped so that the span cannot cover a time before the application started, or after the current time
    fn set_bounded_end_and_shift(&mut self, unbounded_end: Timestamp) {
        let min_end = Timestamp::app_init() + self.span.duration();
        let max_end = Timestamp::now();
        let bounded_end = unbounded_end.max(min_end).min(max_end);
        self.span.set_end_and_shift(bounded_end);
    }

    /// Returns the actual `Span` representing the scope to render
    pub fn to_span(&self) -> Span {
        Span::new(self.span.begin(), self.span.end())
    }

    pub fn zoom_in(&mut self) {
        let new_zoom_level = self.zoom_level.checked_sub(1).unwrap_or(0);
        self.resize(new_zoom_level);
    }

    pub fn zoom_out(&mut self) {
        let now = Timestamp::now();
        let min_begin = min(Timestamp::app_init(), now - DEFAULT_SPAN_DURATION);
        let max_span_duration = now.duration_since(&min_begin);
        let max_units_to_display = max_span_duration.as_secs() as f64 / SPAN_UNIT.as_secs() as f64;
        let max_zoom_level = f64::log2(max_units_to_display).ceil() as u32;

        if self.zoom_level < max_zoom_level {
            self.resize(self.zoom_level + 1);
        }
    }

    fn resize(&mut self, zoom_level: u32) {
        let target_size: Duration = Duration::from_secs(SPAN_UNIT.as_secs() * (1 << zoom_level));
        self.span.set_begin_and_resize(self.span.end() - target_size);
        self.zoom_level = zoom_level;
    }
}

#[cfg(test)]
mod test_rendering_span {
    use std::time::Duration;

    use rstest::*;

    use crate::core::time::test_utils::{
        advance_time_and_refresh_timestamp, setup_fake_clock_to_prevent_substract_overflow,
    };
    use crate::core::time::Timestamp;
    use crate::ctrl::span::RenderingSpan;

    #[fixture]
    fn rendering_span() -> RenderingSpan {
        setup_fake_clock_to_prevent_substract_overflow();
        RenderingSpan::new(Duration::from_secs(60))
    }

    #[rstest]
    fn test_should_end_at_current_timestamp_by_default(rendering_span: RenderingSpan) {
        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_scroll_to_the_right(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.scroll_right();

        let new_span = rendering_span.to_span();

        assert!(new_span.begin() > original_span.begin());
        assert!(new_span.end() > original_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_scroll_past_the_current_timestamp(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        rendering_span.scroll_right();

        let new_span = rendering_span.to_span();

        assert_eq!(original_span.begin(), new_span.begin());
        assert_eq!(original_span.end(), new_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_scroll_to_the_left(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.scroll_left();

        let new_span = rendering_span.to_span();

        assert!(new_span.begin() < original_span.begin());
        assert!(new_span.end() < original_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_scroll_before_the_first_timestamp_of_the_application() {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        let mut rendering_span = RenderingSpan::new(Duration::from_secs(10));
        advance_time_and_refresh_timestamp(Duration::from_secs(10));

        let original_span = rendering_span.to_span();
        // the span starts at the timestamp of the application start
        assert_eq!(original_span.begin(), Timestamp::app_init());

        rendering_span.scroll_left();

        let new_span = rendering_span.to_span();

        assert_eq!(original_span.begin(), new_span.begin());
        assert_eq!(original_span.end(), new_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_follow_when_not_refreshed(rendering_span: RenderingSpan) {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_follow_when_refreshed(mut rendering_span: RenderingSpan) {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.follow();

        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_not_follow_when_span_is_scrolled_left(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.follow();

        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_not_follow_when_span_is_not_scrolled_all_the_way_right(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();
        rendering_span.scroll_left();
        rendering_span.scroll_right();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.follow();

        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_follow_when_span_is_scrolled_back_all_the_way_right(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();
        rendering_span.scroll_right();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.follow();

        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_zoom_in(mut rendering_span: RenderingSpan) {
        let initial_duration = rendering_span.to_span().duration();

        rendering_span.zoom_in();
        let zoomed_in_span = rendering_span.to_span();

        assert!(zoomed_in_span.duration() < initial_duration);
    }

    #[rstest]
    fn test_should_never_cover_less_than_1_second(mut rendering_span: RenderingSpan) {
        for _ in 0..1000 {
            rendering_span.zoom_in();
        }

        assert!(rendering_span.to_span().duration() >= Duration::from_secs(1));
    }

    #[rstest]
    fn test_should_zoom_out(mut rendering_span: RenderingSpan) {
        let initial_duration = rendering_span.to_span().duration();

        rendering_span.zoom_out();

        assert!(rendering_span.to_span().duration() > initial_duration);
    }

    #[rstest]
    fn test_should_not_break_on_infinite_zooming_out(mut rendering_span: RenderingSpan) {
        let initial_duration = rendering_span.to_span().duration();

        for _ in 0..1000 {
            rendering_span.zoom_out();
        }

        assert!(rendering_span.to_span().duration() > 2 * initial_duration);
    }
}
