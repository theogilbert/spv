use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::app::TuiBackend;
use crate::core::process_view::ProcessMetadata;

pub struct MetadataBar {
    current_process: Option<ProcessMetadata>,
}

impl Default for MetadataBar {
    fn default() -> Self {
        Self {
            current_process: None
        }
    }
}

impl MetadataBar {
    fn build_paragraph(&self) -> Paragraph {
        let text = match self.current_process.as_ref() {
            None => "No process is currently selected".to_string(),
            Some(pm) => format!("{} - {}", pm.pid(), pm.command()),
        };

        Paragraph::new(Span::raw(text))
            .style(Style::default().fg(Color::White))
    }

    pub fn set_selected_process(&mut self, process_data: ProcessMetadata) {
        self.current_process = Some(process_data);
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let paragraph = self.build_paragraph();
        frame.render_widget(paragraph, chunk);
    }
}