use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Spans;
use tui::widgets::{Tabs, Widget};
use tui::Frame;
use crate::app::TuiBackend;

pub struct MetricTabs {
    selected_index: usize,
    tabs: Vec<String>,
}

impl MetricTabs {
    pub fn new(metrics_labels: Vec<String>) -> Self {
        Self { selected_index: 0, tabs: metrics_labels }
    }

    pub fn refreshed_tabs(&self) -> Tabs {
        let tabs = self.tabs.iter()
            .cloned()
            .map(Spans::from)
            .collect();

        Tabs::new(tabs)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
            .divider("|")
            .select(self.selected_index)
    }
}