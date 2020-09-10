use tui::widgets::{List, ListItem, ListState, Block, Borders};

use crate::probe::process::{PID, ProcessMetadata};
use tui::style::{Style, Modifier, Color};


pub struct ProcessList{
}

impl ProcessList {
    pub fn new() -> Self {
        Self {}
    }

    pub fn refreshed_list<'a>(&self, processes: &'a [ProcessMetadata]) -> (List<'a>, ListState) {
        let mut state = ListState::default();

        let labels: Vec<ListItem> = processes.iter()
            .map(|pm| ListItem::new(pm.command()))
            .collect();

        let list = List::new(labels)
            .block(Block::default().title("Processes").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC));

        (list, state)
    }
}
