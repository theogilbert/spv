use tui::layout::Alignment;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::Paragraph;

use crate::core::process::{ProcessMetadata, Status};
use crate::ui::labels::relative_timestamp_label;
use crate::ui::terminal::FrameRegion;

pub fn render_metadata_bar(frame: &mut FrameRegion, process: Option<&ProcessMetadata>) {
    match process {
        None => render_no_process_selected(frame),
        Some(pm) => render_process_info(frame, pm),
    };
}

fn render_process_info(frame: &mut FrameRegion, pm: &ProcessMetadata) {
    let left_text = format!("{} ({}) - {}", pm.pid(), pm.status(), pm.command());

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
