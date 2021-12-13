use std::time::Duration;

use crate::core::time::{Span, Timestamp};

pub struct RenderingSpan {
    span: Span,
    follow: bool,
}

impl RenderingSpan {
    pub fn new(duration: Duration, tolerance: Duration) -> Self {
        let mut span = Span::from_duration(duration);
        span.set_tolerance(tolerance);

        Self { span, follow: true }
    }

    pub fn refresh(&mut self) {
        if self.follow {
            self.span.set_end_and_shift(Timestamp::now());
            self.refresh_follow_value();
        }
    }

    pub fn scroll_left(&mut self) {
        self.span.scroll_left(Duration::from_secs(1));
        self.refresh_follow_value();
    }

    pub fn scroll_right(&mut self) {
        self.span.scroll_right(Duration::from_secs(1));
        self.refresh_follow_value();
    }

    pub fn reset_scroll(&mut self) {
        self.span.set_end_and_shift(Timestamp::now());
        self.refresh_follow_value();
    }

    fn refresh_follow_value(&mut self) {
        self.follow = self.span.is_fully_scrolled_right();
    }

    /// Returns the actual `Span` to render
    pub fn span(&self) -> &Span {
        &self.span
    }
}
