use std::io;
use std::time::Duration;

use thiserror::Error;

use crate::core::process::ProcessMetadata;
use crate::core::view::{MetricsOverview, MetricView};
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;
use crate::ui::terminal::Terminal;

mod chart;
mod layout;
mod metadata;
mod processes;
mod tabs;
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
    pub fn new(labels: impl Iterator<Item = String>) -> Result<Self, Error> {
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

    pub fn render(&mut self, metrics_overview: &MetricsOverview, metrics_view: &MetricView) -> Result<(), Error> {
        self.terminal.draw(|mut frame| {
            let layout = UiLayout::new(frame);

            self.tabs.render(&mut frame, layout.tabs_chunk());

            self.process_list
                .render(&mut frame, layout.processes_chunk(), metrics_overview);
            self.chart.render(&mut frame, layout.chart_chunk(), metrics_view);
            self.metadata_bar.render(&mut frame, layout.metadata_chunk());
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

    pub fn current_process(&self) -> Option<&ProcessMetadata> {
        self.process_list.selected()
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
