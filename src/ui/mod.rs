//! Renders spv on the terminal

use std::io;
use std::time::Duration;

use log::error;
use thiserror::Error;

use crate::core::view::{CollectorsView, MetricView, MetricsOverview, ProcessesView};
use crate::ctrl::{Effect, State};
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::sort_processes::render_process_order_popup;
use crate::ui::tabs::render_tabs;
use crate::ui::terminal::Terminal;

mod chart;
mod labels;
mod layout;
mod metadata;
mod processes;
mod sort_processes;
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
    metadata_bar: MetadataBar,
}

impl SpvUI {
    pub fn new(chart_resolution: Duration) -> Result<Self, Error> {
        Ok(Self {
            terminal: Terminal::new()?,
            process_list: ProcessList::default(),
            chart: MetricsChart::new(chart_resolution),
            metadata_bar: MetadataBar::default(),
        })
    }

    pub fn render(
        &mut self,
        collectors: &CollectorsView,
        processes: &ProcessesView,
        overview: &MetricsOverview,
        view: Option<&MetricView>,
        state: State,
    ) -> Result<(), Error> {
        self.terminal.draw(|frame| {
            let layout = UiLayout::new(frame.region());

            render_tabs(frame.with_region(layout.tabs_chunk()), collectors);

            self.process_list
                .render(frame.with_region(layout.processes_chunk()), overview, processes);

            self.chart.render(frame.with_region(layout.chart_chunk()), view);

            self.metadata_bar
                .render(frame.with_region(layout.metadata_chunk()), processes.selected_process());

            if let State::SortingPrompt(criteria) = state {
                render_process_order_popup(frame.with_original_region(), criteria);
            }
        })
    }

    pub fn set_status_from_effect(&mut self, effect: Effect) {
        self.metadata_bar.set_status_from_effect(effect)
    }
}
