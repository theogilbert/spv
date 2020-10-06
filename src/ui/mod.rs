use tui::Frame;

use crate::core::metrics::Archive;
use crate::core::process_view::ProcessMetadata;
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;
use crate::ui::terminal::TuiBackend;
use std::fmt::{Display, Formatter};
use std::fmt;

mod layout;
mod tabs;
mod processes;
mod chart;
mod metadata;
mod terminal;

pub type Terminal = terminal::Terminal;

pub enum Error {
    IOError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let repr = match self {
            Error::IOError(e) => format!("IOError: {}", e),
        };

        write!(f, "{}", repr)
    }
}

pub struct SpvUI {
    tabs: MetricTabs,
    processes: ProcessList,
    chart: MetricsChart,
    metadata_bar: MetadataBar,
}

impl Default for SpvUI {
    fn default() -> Self {
        Self {
            // TODO This is for POC purposes
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            processes: ProcessList::default(),
            chart: MetricsChart::default(),
            metadata_bar: MetadataBar::default(),
        }
    }
}

impl SpvUI {
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>, metrics: &Archive) {
        let layout = UiLayout::new(frame);

        self.tabs.render(frame, layout.tabs_chunk());
        self.processes.render(frame, layout.processes_chunk(), metrics, self.tabs.current());
        self.chart.render(frame, layout.chart_chunk(), self.processes.selected(), metrics);
        self.metadata_bar.render(frame, layout.metadata_chunk());
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        self.processes.set_processes(processes);
        self.metadata_bar.set_selected_process(self.processes.selected());
    }

    pub fn next_process(&mut self) {
        self.processes.next();
        self.metadata_bar.set_selected_process(self.processes.selected());
    }

    pub fn previous_process(&mut self) {
        self.processes.previous();
        self.metadata_bar.set_selected_process(self.processes.selected());
    }

    pub fn current_tab(&self) -> &str {
        self.tabs.current()
    }
}
