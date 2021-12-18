use tui::style::{Color, Style};
use tui::text::Spans;
use tui::widgets::Tabs;

use crate::core::view::CollectorsView;
use crate::ui::terminal::FrameRegion;

pub fn render_tabs(frame: &mut FrameRegion, collectors: &CollectorsView) {
    let tabs_spans = collectors.collectors_names().iter().cloned().map(Spans::from).collect();

    let tabs = Tabs::new(tabs_spans)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
        .divider("|")
        .select(collectors.selected_index());

    frame.render_widget(tabs);
}
