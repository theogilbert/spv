use std::time::Duration;
use std::io;

use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] io::Error),
}

pub struct SpvUI {
    terminal: Terminal,
    tabs: MetricTabs,
    process_list: ProcessList,
    chart: MetricsChart,
    metadata_bar: MetadataBar,
}

impl SpvUI {
    pub fn new(labels: impl Iterator<Item=String>) -> Result<Self, Error> {
        let tabs = MetricTabs::new(labels.collect());
        let chart = MetricsChart::new(Duration::from_secs(60));

        Ok(Self {
            terminal: Terminal::new()?,
            tabs,
            process_list: ProcessList::default(),
            chart,
            metadata_bar: MetadataBar::default(),
        })
    }

    pub fn render(&mut self, metrics: &Archive) -> Result<(), Error> {
        // We need to do this because the borrow checker does not like having &self.foo in a closure
        // while borrowing &mut self.terminal
        let tabs = &self.tabs;
        let process_list = &mut self.process_list;
        let chart = &self.chart;
        let metadata_bar = &self.metadata_bar;

        self.terminal.draw(|mut frame| {
            let layout = UiLayout::new(&frame);

            tabs.render(&mut frame, layout.tabs_chunk());
            process_list.render(&mut frame, layout.processes_chunk(), metrics, tabs.current());
            chart.render(&mut frame, layout.chart_chunk(), process_list.selected(), metrics,
                         tabs.current());
            metadata_bar.render(&mut frame, layout.metadata_chunk());
        })
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

    pub fn next_tab(&mut self) {
        self.tabs.next();
    }

    pub fn previous_tab(&mut self) {
        self.tabs.previous();
    }
}

