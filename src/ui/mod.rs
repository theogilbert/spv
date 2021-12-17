use std::io;
use std::time::Duration;

use log::error;
use thiserror::Error;

use crate::core::view::{CollectorsView, MetricView, MetricsOverview, ProcessesView};
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::render_metadata_bar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::render_tabs;
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
    process_list: ProcessList,
    chart: MetricsChart,
}

impl SpvUI {
    pub fn new(chart_resolution: Duration) -> Result<Self, Error> {
        Ok(Self {
            terminal: Terminal::new()?,
            process_list: ProcessList::default(),
            chart: MetricsChart::new(chart_resolution),
        })
    }

    pub fn render(
        &mut self,
        collectors: &CollectorsView,
        processes: &ProcessesView,
        overview: &MetricsOverview,
        view: Option<&MetricView>,
    ) -> Result<(), Error> {
        self.terminal.draw(|frame| {
            let layout = UiLayout::new(frame.region());

            render_tabs(frame.with_region(layout.tabs_chunk()), collectors);

            self.process_list
                .render(frame.with_region(layout.processes_chunk()), overview, processes);

            self.chart.render(frame.with_region(layout.chart_chunk()), view);

            render_metadata_bar(frame.with_region(layout.metadata_chunk()), processes.selected_process());
        })
    }
}
