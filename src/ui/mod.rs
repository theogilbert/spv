use std::io;
use std::time::Duration;

use log::error;
use thiserror::Error;

use crate::core::view::{MetricView, MetricsOverview, ProcessView};
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;
use crate::ui::terminal::Terminal;

mod chart;
mod labels;
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
    pub fn new(labels: impl Iterator<Item = String>, chart_resolution: Duration) -> Result<Self, Error> {
        let tabs = MetricTabs::new(labels.collect());

        Ok(Self {
            terminal: Terminal::new()?,
            tabs,
            process_list: ProcessList::default(),
            chart: MetricsChart::new(chart_resolution),
            metadata_bar: MetadataBar::default(),
        })
    }

    pub fn render(
        &mut self,
        overview: &MetricsOverview,
        view: &Option<MetricView>,
        processes: &ProcessView,
    ) -> Result<(), Error> {
        self.terminal.draw(|frame| {
            let layout = UiLayout::new(frame.region());

            self.tabs.render(frame.with_region(layout.tabs_chunk()));

            self.process_list
                .render(frame.with_region(layout.processes_chunk()), overview, processes);

            self.chart.render(frame.with_region(layout.chart_chunk()), view);

            self.metadata_bar
                .render(frame.with_region(layout.metadata_chunk()), processes.selected_process());
        })
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
