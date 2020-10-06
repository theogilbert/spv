use std::fmt::{Display, Formatter};
use std::fmt;

use crate::core::metrics::Archive;
use crate::core::process_view::ProcessMetadata;
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;
use crate::ui::terminal::Terminal;

mod layout;
mod tabs;
mod processes;
mod chart;
mod metadata;
mod terminal;

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
    terminal: Terminal,
    tabs: MetricTabs,
    process_list: ProcessList,
    chart: MetricsChart,
    metadata_bar: MetadataBar,
}

impl SpvUI {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            terminal: Terminal::new()?,
            // TODO This is for POC purposes
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            process_list: ProcessList::default(),
            chart: MetricsChart::default(),
            metadata_bar: MetadataBar::default(),
        })
    }

    pub fn render(&mut self, metrics: &Archive) -> Result<(), Error> {
        // We need to do this because the borrow checker does not like having &self.foo in a closure
        // while borrowing &mut self.terminal
        let tabs = &self.tabs;
        let mut process_list = &mut self.process_list;
        let chart = &self.chart;
        let metadata_bar = &self.metadata_bar;

        self.terminal.draw(|mut frame| {
            let layout = UiLayout::new(&mut frame);

            tabs.render(&mut frame, layout.tabs_chunk());
            process_list.render(&mut frame, layout.processes_chunk(), metrics, tabs.current());
            chart.render(&mut frame, layout.chart_chunk(), process_list.selected(), metrics);
            metadata_bar.render(&mut frame, layout.metadata_chunk());
        }).map_err(|e| Error::IOError(e.to_string()))
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        self.process_list.set_processes(processes);
        self.metadata_bar.set_selected_process(self.process_list.selected());
    }

    pub fn next_process(&mut self) {
        self.process_list.next();
        self.metadata_bar.set_selected_process(self.process_list.selected());
    }

    pub fn previous_process(&mut self) {
        self.process_list.previous();
        self.metadata_bar.set_selected_process(self.process_list.selected());
    }

    pub fn current_tab(&self) -> &str {
        self.tabs.current()
    }
}

