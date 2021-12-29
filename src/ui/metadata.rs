use std::time::Duration;

use tui::layout::Alignment;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::core::ordering::ProcessOrdering;
use crate::core::process::{ProcessMetadata, Status};
use crate::core::time::Timestamp;
use crate::ctrl::Effect;
use crate::ui::labels::{process_criteria_label, relative_timestamp_label};
use crate::ui::layout::centered_area;
use crate::ui::terminal::FrameRegion;

const STATUS_DISPLAY_TIME: Duration = Duration::from_secs(2);

pub struct MetadataBar {
    status: Effect,
    date_of_status: Timestamp,
}

impl Default for MetadataBar {
    fn default() -> Self {
        Self {
            status: Effect::None,
            date_of_status: Timestamp::app_init(),
        }
    }
}

impl MetadataBar {
    pub fn set_status_from_effect(&mut self, effect: Effect) {
        self.status = effect;
        self.date_of_status = Timestamp::from_current_instant();
    }

    pub fn render(&mut self, frame: &mut FrameRegion, process: Option<&ProcessMetadata>) {
        self.refresh_status();

        let original_area = frame.region();
        let area_with_margin = centered_area(
            original_area,
            original_area.width.saturating_sub(2),
            original_area.height,
        );

        match self.status {
            Effect::None => render_process_metadata(frame.with_region(area_with_margin), process),
            Effect::ProcessesSorted(criteria) => {
                render_process_sorted_status(frame.with_region(area_with_margin), criteria)
            }
        }
    }

    fn refresh_status(&mut self) {
        if self.date_of_status + STATUS_DISPLAY_TIME < Timestamp::from_current_instant() {
            self.status = Effect::None;
        }
    }
}

fn render_process_metadata(frame: &mut FrameRegion, process: Option<&ProcessMetadata>) {
    match process {
        None => render_no_process_selected(frame),
        Some(pm) => render_process_info(frame, pm),
    };
}

fn render_process_info(frame: &mut FrameRegion, pm: &ProcessMetadata) {
    let left_text = format!("{} - {}", pm.pid(), pm.command());

    let begin_time = relative_timestamp_label(pm.running_span().begin());
    let mut right_text = format!("Started {}", begin_time);

    if pm.status() == Status::DEAD {
        let end_time = relative_timestamp_label(pm.running_span().end());
        right_text.push_str(&format!(" - Dead {}", end_time));
    }

    let should_draw_right_paragraph = frame.region().width as usize > left_text.len() + right_text.len();

    let left_paragraph = Paragraph::new(Span::from(left_text)).style(Style::default().fg(Color::White));
    let right_paragraph = Paragraph::new(Span::from(right_text))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Right);

    frame.render_widget(left_paragraph);
    if should_draw_right_paragraph {
        frame.render_widget(right_paragraph);
    }
}

fn render_no_process_selected(frame: &mut FrameRegion) {
    let left_text = "No process is currently selected";
    let paragraph = Paragraph::new(Span::raw(left_text)).style(Style::default().fg(Color::White));
    frame.render_widget(paragraph);
}

fn render_process_sorted_status(frame: &mut FrameRegion, criteria: ProcessOrdering) {
    let text = format!(
        "Processes sorted by {}",
        process_criteria_label(&criteria).to_lowercase()
    );
    let paragraph = Paragraph::new(Span::from(text)).style(Style::default().fg(Color::Black).bg(Color::White));
    frame.render_widget(paragraph);
}
