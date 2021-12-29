use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::text::Spans;
use tui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::core::ordering::{ProcessOrdering, PROCESS_ORDERING_CRITERIA};
use crate::ui::labels::process_criteria_label;
use crate::ui::layout::centered_area;
use crate::ui::terminal::FrameRegion;

pub fn render_process_order_popup(frame_region: &mut FrameRegion, selected_criteria: ProcessOrdering) {
    const POPUP_WIDTH: u16 = 50;
    // Why +5 -> 3 for borders (top, middle, bottom) + 2 for criteria description:
    const POPUP_HEIGHT: u16 = PROCESS_ORDERING_CRITERIA.len() as u16 + 5;

    let popup_area = centered_area(frame_region.region(), POPUP_WIDTH, POPUP_HEIGHT);

    frame_region.with_region(popup_area).render_widget(Clear);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(PROCESS_ORDERING_CRITERIA.len() as u16),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(popup_area);

    let popup_block = Block::default().borders(Borders::ALL).title("Sort processes");
    frame_region.with_region(popup_area).render_widget(popup_block);

    render_selection_list(
        frame_region.with_region(chunks[0]),
        &PROCESS_ORDERING_CRITERIA,
        &selected_criteria,
    );
    render_selection_description(frame_region.with_region(chunks[1]), &selected_criteria);
}

fn render_selection_list(
    frame_region: &mut FrameRegion,
    criteria: &[ProcessOrdering],
    selected_criteria: &ProcessOrdering,
) {
    let selected_index = criteria
        .iter()
        .position(|c| c == selected_criteria)
        .expect("A criteria is not covered by the process order widget");

    let mut state = ListState::default();
    state.select(Some(selected_index));

    let texts: Vec<_> = criteria.iter().map(process_criteria_label).collect();
    let max_text_length = texts.iter().map(|t| t.len()).max().unwrap() as u16;

    let items: Vec<_> = texts.into_iter().map(ListItem::new).collect();
    let list = List::new(items).highlight_symbol(">> ");

    let list_area = centered_area(frame_region.region(), max_text_length + 3, frame_region.region().height);
    frame_region
        .with_region(list_area)
        .render_stateful_widget(list, &mut state);
}

fn render_selection_description(frame_region: &mut FrameRegion, selected_criteria: &ProcessOrdering) {
    let text = match selected_criteria {
        ProcessOrdering::CurrentMetric => "Order processes by their last collected metric, in a descending order",
        ProcessOrdering::Pid => "Order processes by their pid, in an ascending order",
        ProcessOrdering::Command => "Order processes by their command, in an alphabetically ascending order",
    }
    .to_string();

    let paragraph = Paragraph::new(Spans::from(text))
        .block(Block::default().borders(Borders::TOP))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame_region.render_widget(paragraph);
}
