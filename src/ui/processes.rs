use tui::Frame;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::TuiBackend;
use crate::core::process_view::{PID, ProcessMetadata};

pub struct ProcessList {
    processes: Vec<ProcessMetadata>,
    selected_pid: Option<PID>,
    state: ListState,
}

impl Default for ProcessList {
    fn default() -> Self {
        Self {
            processes: vec![],
            selected_pid: None,
            state: ListState::default(),
        }
    }
}

impl ProcessList {
    pub fn render<'a>(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let items: Vec<ListItem> = self.processes.iter()
            .map(|pm| ListItem::new(pm.command()))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        self.processes = processes;

        if self.processes.is_empty() {
            self.select(None);
        } else {
            let mut selected_idx = 0;

            if let Some(selected_pid) = self.selected_pid {
                selected_idx = self.processes.iter()
                    .position(|pm| pm.pid() == selected_pid)
                    .unwrap_or(0); // If PID does not exist anymore, select first process
            }

            self.select(Some(selected_idx));
        }
    }

    fn select(&mut self, index: Option<usize>) {
        match index {
            None => {
                self.state.select(None);
                self.selected_pid = None
            }
            Some(i) => {
                if let Some(pm) = self.processes.get(i) {
                    self.state.select(index);
                    self.selected_pid = Some(pm.pid());
                }
            }
        }
    }

    pub fn previous(&mut self) {
        let prev_idx = self.state.selected()
            .and_then(|s| Some(if s > 0 { s - 1 } else { 0 }))
            .unwrap_or(0);

        self.select(Some(prev_idx));
    }

    pub fn next(&mut self) {
        let next_idx = self.state.selected()
            .and_then(|s| Some(s + 1))
            .unwrap_or(0);

        self.select(Some(next_idx));
    }

    pub fn selected(&self) -> Option<&ProcessMetadata> {
        match self.selected_pid {
            None => None,
            Some(pid) => self.processes.iter().find(|pm| pm.pid() == pid)
        }
    }
}
