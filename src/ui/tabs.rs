use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Spans;
use tui::widgets::Tabs;

use crate::ui::terminal::TuiBackend;

pub struct MetricTabs {
    selected_index: usize,
    tabs: Vec<String>,
}

impl MetricTabs {
    pub fn new(metrics_labels: Vec<String>) -> Self {
        Self { selected_index: 0, tabs: metrics_labels }
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let tabs_spans = self.tabs.iter()
            .cloned()
            .map(Spans::from)
            .collect();

        let tabs = Tabs::new(tabs_spans)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
            .divider("|")
            .select(self.selected_index);

        frame.render_widget(tabs, chunk);
    }

    pub fn current(&self) -> &str {
        self.tabs.get(self.selected_index)
            .unwrap_or_else(|| panic!("Invalid tab index: {}", self.selected_index))
    }

    pub fn next(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.tabs.len();
    }

    pub fn previous(&mut self) {
        self.selected_index = self.selected_index.checked_sub(1)
            .unwrap_or(self.tabs.len() - 1);
    }
}