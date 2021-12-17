use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::core::process::{Pid, ProcessMetadata, Status};
use crate::core::view::{MetricsOverview, ProcessesView};
use crate::ui::terminal::FrameRegion;

/// Width of the process name column
const CMD_COL_WIDTH: usize = 16;
/// Width of the metrics values column
const METRICS_COL_WIDTH: usize = 10;

#[derive(Default)]
pub struct ProcessList {
    state: ListState,
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
    pub fn render(&mut self, frame: &mut FrameRegion, metrics_overview: &MetricsOverview, processes: &ProcessesView) {
        self.state.select(processes.selected_index());

        let rows_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(frame.region());

        let (proc_chunk, metric_chunk) = Self::split_column_chunks(rows_chunks[1]);

        Self::render_title_row(frame.with_region(rows_chunks[0]), metrics_overview.unit());
        self.render_name_column(frame.with_region(proc_chunk), processes.as_slice());
        self.render_metric_column(frame.with_region(metric_chunk), metrics_overview, processes.as_slice());
    }

    /// Splits a `Rect` into two:
    ///   - One that will contain the command name
    ///   - One that will contain the metric value
    fn split_column_chunks(chunk: Rect) -> (Rect, Rect) {
        let columns_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(CMD_COL_WIDTH as u16 + 2), // processes names
                    Constraint::Min(METRICS_COL_WIDTH as u16), // processes metrics
                ]
                .as_ref(),
            )
            .split(chunk);

        (columns_chunks[0], columns_chunks[1])
    }

    fn render_title_row(frame: &mut FrameRegion, metric_unit: &'static str) {
        let (proc_chunk, metric_chunk) = Self::split_column_chunks(frame.region());

        let proc_paragraph = Paragraph::new("Process name")
            .block(Block::default().borders(Borders::LEFT | Borders::TOP))
            .alignment(Alignment::Center);

        let metric_text = format!("{} ", metric_unit);
        let metric_title = Paragraph::new(metric_text)
            .block(Block::default().borders(Borders::TOP))
            .alignment(Alignment::Right);

        frame.with_region(proc_chunk).render_widget(proc_paragraph);
        frame.with_region(metric_chunk).render_widget(metric_title);
    }

    fn render_name_column(&mut self, frame: &mut FrameRegion, processes: &[ProcessMetadata]) {
        let processes_names: Vec<_> = processes.iter().map(Self::shortened_command_name).collect();

        let items: Vec<ListItem> = processes_names.iter().map(|cmd| ListItem::new(cmd.as_str())).collect();

        let list = Self::build_default_list_widget(items)
            .block(Block::default().borders(Borders::LEFT | Borders::BOTTOM))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, &mut self.state);
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

    fn render_metric_column(
        &mut self,
        frame: &mut FrameRegion,
        metrics_overview: &MetricsOverview,
        processes: &[ProcessMetadata],
    ) {
        let str_metrics: Vec<String> = processes
            .iter()
            .map(|pm| match pm.status() {
                Status::RUNNING => self.formatted_process_metric(pm.pid(), metrics_overview),
                Status::DEAD => self.justify_metric_repr("DEAD".to_string()),
            })
            .collect();

        let items: Vec<ListItem> = str_metrics.iter().map(|m| ListItem::new(m.as_str())).collect();

        let list = Self::build_default_list_widget(items).block(Block::default().borders(Borders::BOTTOM));

        frame.render_stateful_widget(list, &mut self.state);
    }

    fn formatted_process_metric(&self, pid: Pid, metrics_overview: &MetricsOverview) -> String {
        let m = metrics_overview.last_or_default(pid);
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

    use crate::ui::processes::{ProcessList, METRICS_COL_WIDTH};

    #[fixture]
    fn process_list() -> ProcessList {
        ProcessList::default()
    }

    #[fixture]
    fn short_metric_repr() -> String {
        std::iter::repeat('0').take(METRICS_COL_WIDTH / 2).collect()
    }

    #[rstest(input, case("a"), case("abcdefgh"))]
    fn test_should_align_right_with_right_padding(process_list: ProcessList, input: &str) {
        let aligned = process_list.justify_metric_repr(input.to_string());

        assert!(aligned.ends_with(&format!("{} ", input)));
        assert_eq!(aligned.len(), METRICS_COL_WIDTH)
    }

    #[rstest]
    fn test_should_contain_one_extra_space_in_front_of_short_text(
        process_list: ProcessList,
        short_metric_repr: String,
    ) {
        let justified_repr = process_list.justify_metric_repr(short_metric_repr);
        assert!(justified_repr.starts_with(" "));
    }

    #[rstest]
    fn test_should_add_trailing_space_on_short_repr(process_list: ProcessList, short_metric_repr: String) {
        let justified_repr = process_list.justify_metric_repr(short_metric_repr);
        assert!(justified_repr.ends_with(" "));
    }
}
