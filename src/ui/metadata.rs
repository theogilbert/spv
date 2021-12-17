use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::core::process::ProcessMetadata;
use crate::ui::terminal::FrameRegion;

#[derive(Default)]
pub struct MetadataBar;

impl MetadataBar {
    pub fn render(&self, frame: &mut FrameRegion, process: Option<&ProcessMetadata>) {
        let text = match process.as_ref() {
            None => "No process is currently selected".to_string(),
            Some(pm) => {
                format!("{} ({}) - {}", pm.pid(), pm.status(), pm.command())
            }
        };

        let paragraph = Paragraph::new(Span::from(text)).style(Style::default().fg(Color::White));
        frame.render_widget(paragraph);
    }
}
