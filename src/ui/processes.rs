use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::TuiBackend;
use crate::core::process_view::ProcessMetadata;

pub struct ProcessList {}

impl ProcessList {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render<'a>(&self, frame: &mut Frame<TuiBackend>, chunk: Rect, processes: &'a [ProcessMetadata]) {
        let mut state = ListState::default();

        let labels: Vec<ListItem> = processes.iter()
            .map(|pm| ListItem::new(pm.command()))
            .collect();

        let list = List::new(labels)
            .block(Block::default().title("Processes").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC));

        frame.render_stateful_widget(list, chunk, &mut state);
    }
}
