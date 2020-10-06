use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::core::process_view::ProcessMetadata;
use crate::ui::terminal::TuiBackend;

pub struct MetadataBar {
    current_text: String,
}

impl Default for MetadataBar {
    fn default() -> Self {
        Self {
            current_text: Self::build_text(None)
        }
    }
}

impl MetadataBar {
    fn build_text(process: Option<&ProcessMetadata>) -> String {
        match process.as_ref() {
            None => "No process is currently selected".to_string(),
            Some(pm) => format!("{} - {}", pm.pid(), pm.command()),
        }
    }

    fn build_paragraph(&self) -> Paragraph {
        Paragraph::new(Span::raw(self.current_text.as_str()))
            .style(Style::default().fg(Color::White))
    }

    pub fn set_selected_process(&mut self, process_data: Option<&ProcessMetadata>) {
        self.current_text = Self::build_text(process_data);
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let paragraph = self.build_paragraph();
        frame.render_widget(paragraph, chunk);
    }
}