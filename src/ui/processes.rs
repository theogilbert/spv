use tui::Frame;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::TuiBackend;
use crate::core::metrics::Archive;
use crate::core::process_view::{PID, ProcessMetadata};

pub struct ProcessList {
    processes: Vec<ProcessMetadata>,
    selected_pid: Option<PID>,
    state: ListState,
    metrics_col_len: u16,
}

impl Default for ProcessList {
    fn default() -> Self {
        Self {
            processes: vec![],
            selected_pid: None,
            state: ListState::default(),
            metrics_col_len: 6,
        }
    }
}

// TODO add first row Process name - [%/bps]
impl ProcessList {
    pub fn render<'a>(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, metrics: &Archive,
                      label: &str) {
        let columns_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(1),  // processes names
                    Constraint::Length(self.metrics_col_len),  // processes metrics
                ].as_ref()
            )
            .split(chunk);

        self.render_name_column(frame, columns_chunks[0]);
        self.render_metric_column(frame, columns_chunks[1], metrics, label);
    }

    fn render_name_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let items: Vec<ListItem> = self.processes.iter()
            .map(|pm| ListItem::new(pm.command()))
            .collect();

        let list = Self::build_list(items)
            .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    fn render_metric_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, metrics: &Archive, label: &str) {
        let metrics_values: Vec<String> = self.processes.iter()
            .map(|pm| {  // build String from metric value
                metrics.current(label, pm.pid())
                    .expect("Error getting current metric")
                    .to_string()
            })
            .map(|s| self.align_metric_right(s))
            .collect();

        let items: Vec<ListItem> = metrics_values.iter()
            .map(|s| ListItem::new(s.as_str()))
            .collect();

        let list = Self::build_list(items)
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM));

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    fn align_metric_right(&self, text: String) -> String {
        let indent = (self.metrics_col_len as usize).checked_sub(text.len() + 1)
            .unwrap_or(1)
            .max(1);
        format!("{:indent$}{} ", "", text, indent = indent)
    }

    fn build_list(items: Vec<ListItem>) -> List {
        List::new(items)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
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


#[cfg(test)]
mod test_align_right {
    use rstest::*;

    use crate::ui::processes::ProcessList;

    #[fixture]
    fn process_list() -> ProcessList {
        let mut pl = ProcessList::default();
        pl.metrics_col_len = 10;
        pl
    }

    #[rstest(input,
    case("a"),
    case("ab"),
    case("abc"),
    case("abcd"),
    case("abcde"),
    case("abcdef"),
    case("abcdefg"),
    case("abcdefgh"),
    )]
    fn test_should_align_right_with_padding(process_list: ProcessList, input: &str) {
        let aligned = process_list.align_metric_right(input.to_string());

        assert!(aligned.ends_with(&format!("{} ", input)));
        assert_eq!(aligned.len(), process_list.metrics_col_len as usize)
    }

    #[rstest]
    fn test_should_contain_one_extra_space_in_front_of_long_text(process_list: ProcessList) {
        let aligned = process_list.align_metric_right("012345678910".to_string());
        assert_eq!(aligned, " 012345678910 ");
    }
}