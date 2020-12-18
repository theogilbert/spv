use tui::Frame;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::core::metrics::Archive;
use crate::core::process_view::{PID, ProcessMetadata};
use crate::ui::terminal::TuiBackend;

/// Maximum amount of characters to display for a command (including ".." if the cmd is too long)
const MAX_COMMAND_LENGTH: usize = 16;
/// Maximum amount of characters to display for a metric
const MAX_METRICS_LENGTH: usize = 10;

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

// TODO add first row Process name - [%/bps]
impl ProcessList {
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, metrics: &Archive,
                  label: &str) {
        let columns_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(MAX_COMMAND_LENGTH as u16 + 2),  // processes names
                    Constraint::Length(MAX_METRICS_LENGTH as u16),  // processes metrics
                ].as_ref()
            )
            .split(chunk);

        self.render_name_column(frame, columns_chunks[0]);
        self.render_metric_column(frame, columns_chunks[1], metrics, label);
    }

    fn shortened_command_name(process_metadata: &ProcessMetadata) -> String {
        if process_metadata.command().len() > MAX_COMMAND_LENGTH {
            format!("{}..", &process_metadata.command()[0..MAX_COMMAND_LENGTH - 2])
        } else {
            process_metadata.command().to_string()
        }
    }

    fn render_name_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let processes_names: Vec<_> = self.processes.iter()
            .map(|pm| Self::shortened_command_name(pm))
            .collect();

        let items: Vec<ListItem> = processes_names.iter()
            .map(|cmd| ListItem::new(cmd.as_str()))
            .collect();

        let list = Self::build_default_list_widget(items)
            .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    fn render_metric_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, metrics: &Archive, label: &str) {
        let str_metrics: Vec<String> = self.processes.iter()
            .map(|pm| self.formatted_process_metric(pm, metrics, label))
            .collect();

        let items: Vec<ListItem> = str_metrics.iter()
            .map(|pm| ListItem::new(pm.as_str()))
            .collect();

        let list = Self::build_default_list_widget(items)
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM));

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    fn formatted_process_metric(&self, process: &ProcessMetadata, metrics: &Archive,
                                label: &str) -> String {
        let m = metrics.last(label, process.pid())
            .expect("Error getting current metric");

        self.align_metric_right(m.concise_repr())
    }

    fn align_metric_right(&self, text: String) -> String {
        let indent = MAX_METRICS_LENGTH.checked_sub(text.len() + 1)
            .unwrap_or(1)
            .max(1);
        format!("{:indent$}{} ", "", text, indent = indent)
    }

    fn build_default_list_widget(items: Vec<ListItem>) -> List {
        List::new(items)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        let index_opt = if self.processes.is_empty() {
            None
        } else {
            Some(Self::retrieve_previously_selected_index(&processes, self.selected_pid))
        };

        self.processes = processes;
        self.select(index_opt);
    }

    fn retrieve_previously_selected_index(processes: &[ProcessMetadata],
                                          selected_pid: Option<PID>) -> usize {
        let mut selected_idx = 0;

        if let Some(selected_pid) = selected_pid {
            selected_idx = processes.iter()
                .position(|pm| pm.pid() == selected_pid)
                .unwrap_or(0); // If PID does not exist anymore, select first process
        }

        selected_idx
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
            .map(|s| if s > 0 { s - 1 } else { 0 })
            .unwrap_or(0);

        self.select(Some(prev_idx));
    }

    pub fn next(&mut self) {
        let next_idx = self.state.selected()
            .map(|s| s + 1)
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

    use crate::ui::processes::{ProcessList, MAX_METRICS_LENGTH};

    #[fixture]
    fn process_list() -> ProcessList {
        ProcessList::default()
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
    fn test_should_align_right_with_right_padding(process_list: ProcessList, input: &str) {
        let aligned = process_list.align_metric_right(input.to_string());

        assert!(aligned.ends_with(&format!("{} ", input)));
        assert_eq!(aligned.len(), MAX_METRICS_LENGTH)
    }

    #[rstest]
    fn test_should_contain_one_extra_space_in_front_of_long_text(process_list: ProcessList) {
        let aligned = process_list.align_metric_right("012345678910".to_string());
        assert_eq!(aligned, " 012345678910 ");
    }
}