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

#[cfg(test)]
mod test_tabs {
    use tui::buffer::Buffer;
    use tui::layout::Rect;
    use tui::style::{Color, Style};

    use crate::core::view::CollectorsView;
    use crate::ui::tabs::render_tabs;
    use crate::ui::terminal::Terminal;

    #[test]
    fn should_render_all_collectors_names() {
        let mut terminal = Terminal::from_size(40, 1).unwrap();
        let view = CollectorsView::new(vec!["collector_1", "collector_2"], 1);

        terminal.draw(|fr| render_tabs(fr, &view)).unwrap();

        let mut expected_buffer = Buffer::with_lines(vec![" collector_1 | collector_2              "]);
        expected_buffer.set_style(expected_buffer.area, Style::default().fg(Color::White));
        expected_buffer.set_style(
            Rect::new(15, 0, 11, 1), // collector_2 is selected, and thus highlighted
            Style::default().bg(Color::White).fg(Color::Black),
        );

        terminal.assert_buffer(expected_buffer)
    }
}
