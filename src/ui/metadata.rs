use tui::text::Span;
use tui::widgets::Paragraph;

use crate::probe::process::ProcessMetadata;
use tui::style::{Style, Color};

pub struct MetadataBar;

impl Default for MetadataBar {
    fn default() -> Self {
        Self {}
    }
}

impl MetadataBar {
    pub fn refreshed_metadata<'a>(&self) -> Paragraph {
        Paragraph::new(Span::raw("34951 z3tyop ping www.google.fr"))
            .style(Style::default().fg(Color::White))
    }
}