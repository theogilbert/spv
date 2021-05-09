use tui::Frame;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::core::metrics::Archive;
use crate::core::process_view::{PID, ProcessMetadata};
use crate::ui::terminal::TuiBackend;

/// Width of the process name column
const CMD_COL_WIDTH: usize = 16;
/// Width of the metrics values column
const METRICS_COL_WIDTH: usize = 10;

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
    /// Renders the processes assigned through the
    /// [`set_processes`](struct.ProcessList#method.set_processes) method
    ///
    /// # Arguments
    ///   * `frame`: The `Frame` on which to render the process list widget
    ///   * `chunk`: The region within the `frame` reserved for this widget
    ///   * `archive`: The metrics archive, to display the current metric of each process
    ///   * `label`: The name of the metric to display
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, archive: &Archive,
                  label: &str) {
        let rows_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(chunk);

        let (proc_chunk, metric_chunk) = Self::split_column_chunks(rows_chunks[1]);

        Self::render_title_row(frame, rows_chunks[0], archive, label);
        self.render_name_column(frame, proc_chunk);
        self.render_metric_column(frame, metric_chunk, archive, label);
    }

    /// Define the processes to render in the process list
    /// The processes will be displayed in the list in the same order as they appear in `processes`
    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        let index_opt = if self.processes.is_empty() {
            None
        } else {
            Some(Self::retrieve_index_of_previously_selected_pid(&processes, self.selected_pid))
        };

        self.processes = processes;
        self.select(index_opt);
    }

    /// Focus the previous process
    pub fn previous(&mut self) {
        let prev_idx = self.state.selected()
            .map(|s| if s > 0 { s - 1 } else { 0 })
            .unwrap_or(0);

        self.select(Some(prev_idx));
    }

    /// Focus the next process
    pub fn next(&mut self) {
        let next_idx = self.state.selected()
            .map(|s| s + 1)
            .unwrap_or(0);

        self.select(Some(next_idx));
    }

    /// Returns the selected `&ProcessMetadata`
    pub fn selected(&self) -> Option<&ProcessMetadata> {
        match self.selected_pid {
            None => None,
            Some(pid) => self.processes.iter().find(|pm| pm.pid() == pid)
        }
    }

    /// Select the process at the given index
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


    /// Processes are displayed in a list, sorted by their metric values
    /// From one frame to the other, the same process may have a different position in the list
    /// This function returns the new position of the selected process in the given `processes` list
    fn retrieve_index_of_previously_selected_pid(processes: &[ProcessMetadata],
                                                 selected_pid: Option<PID>) -> usize {
        match selected_pid {
            Some(selected_pid) => {
                processes.iter()
                    .position(|pm| pm.pid() == selected_pid)
                    .unwrap_or(0) // If PID does not exist anymore, select first process
            }
            None => 0
        }
    }

    /// Splits a `Rect` into two:
    ///   - One that will contain the command name
    ///   - One that will contain the metric value
    fn split_column_chunks(chunk: Rect) -> (Rect, Rect) {
        let columns_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(CMD_COL_WIDTH as u16 + 2),  // processes names
                    Constraint::Min(METRICS_COL_WIDTH as u16),  // processes metrics
                ].as_ref()
            )
            .split(chunk);

        (columns_chunks[0], columns_chunks[1])
    }

    fn render_title_row(frame: &mut Frame<TuiBackend>, chunk: Rect, archive: &Archive, label: &str) {
        let (proc_chunk, metric_chunk) = Self::split_column_chunks(chunk);

        let proc_paragraph = Paragraph::new("Process name")
            .block(Block::default().borders(Borders::LEFT | Borders::TOP))
            .alignment(Alignment::Center);

        let metric_text = format!("{} ", archive.label_unit(label).unwrap());
        let metric_title = Paragraph::new(metric_text)
            .block(Block::default().borders(Borders::TOP))
            .alignment(Alignment::Right);

        frame.render_widget(proc_paragraph, proc_chunk);
        frame.render_widget(metric_title, metric_chunk);
    }

    fn render_name_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let processes_names: Vec<_> = self.processes.iter()
            .map(|pm| Self::shortened_command_name(pm))
            .collect();

        let items: Vec<ListItem> = processes_names.iter()
            .map(|cmd| ListItem::new(cmd.as_str()))
            .collect();

        let list = Self::build_default_list_widget(items)
            .block(Block::default().borders(Borders::LEFT | Borders::BOTTOM))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }


    /// Returns the formatted command name of `process_metadata` so that its length does not exceed
    /// `MAX_COMMAND_LENGTH` characters
    fn shortened_command_name(process_metadata: &ProcessMetadata) -> String {
        if process_metadata.command().len() > CMD_COL_WIDTH {
            format!("{}..", &process_metadata.command()[0..CMD_COL_WIDTH - 2])
        } else {
            process_metadata.command().to_string()
        }
    }

    fn render_metric_column(&mut self, frame: &mut Frame<TuiBackend>, chunk: Rect, archive: &Archive, label: &str) {
        let str_metrics: Vec<String> = self.processes.iter()
            .map(|pm| self.formatted_process_metric(pm, archive, label))
            .collect();

        let items: Vec<ListItem> = str_metrics.iter()
            .map(|pm| ListItem::new(pm.as_str()))
            .collect();

        let list = Self::build_default_list_widget(items)
            .block(Block::default().borders(Borders::BOTTOM));

        frame.render_stateful_widget(list, chunk, &mut self.state);
    }

    fn formatted_process_metric(&self, process: &ProcessMetadata, metrics: &Archive,
                                label: &str) -> String {
        let m = metrics.last(label, process.pid())
            .expect("Error getting current metric");
        self.justify_metric_repr(m.concise_repr())
    }

    fn justify_metric_repr(&self, metric_repr: String) -> String {
        format!("{:>width$} ", metric_repr, width = METRICS_COL_WIDTH - 1) // - 1 because of the trailing space
    }

    fn build_default_list_widget(items: Vec<ListItem>) -> List {
        List::new(items)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    }
}


#[cfg(test)]
mod test_justify_right {
    use rstest::*;

    use crate::ui::processes::{METRICS_COL_WIDTH, ProcessList};

    #[fixture]
    fn process_list() -> ProcessList {
        ProcessList::default()
    }

    #[fixture]
    fn short_metric_repr() -> String {
        std::iter::repeat('0')
            .take(METRICS_COL_WIDTH / 2)
            .collect()
    }

    #[rstest(input,
    case("a"),
    case("abcdefgh"),
    )]
    fn test_should_align_right_with_right_padding(process_list: ProcessList, input: &str) {
        let aligned = process_list.justify_metric_repr(input.to_string());

        assert!(aligned.ends_with(&format!("{} ", input)));
        assert_eq!(aligned.len(), METRICS_COL_WIDTH)
    }

    #[rstest]
    fn test_should_contain_one_extra_space_in_front_of_short_text(process_list: ProcessList,
                                                                  short_metric_repr: String) {
        let justified_repr = process_list.justify_metric_repr(short_metric_repr);
        assert!(justified_repr.starts_with(" "));
    }

    #[rstest]
    fn test_should_add_trailing_space_on_short_repr(process_list: ProcessList,
                                                    short_metric_repr: String) {
        let justified_repr = process_list.justify_metric_repr(short_metric_repr);
        assert!(justified_repr.ends_with(" "));
    }
}