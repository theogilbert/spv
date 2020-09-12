use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::app::TuiBackend;

pub struct MetadataBar;

impl Default for MetadataBar {
    fn default() -> Self {
        Self {}
    }
}

impl MetadataBar {
    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let text = "34951 z3tyop ping www.google.fr";
        let paragraph = Paragraph::new(Span::raw(text))
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, chunk);
    }
}