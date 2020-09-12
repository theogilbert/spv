use tui::Frame;

use crate::app::TuiBackend;
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;
use crate::core::process_view::ProcessMetadata;

// Tabs, ProcessList etc... should not leak. FrameRenderer will have next_tab() etc... methods
mod layout;
mod tabs;
mod processes;
mod chart;
mod metadata;

pub struct SpvUI {
    tabs: MetricTabs,
    processes: ProcessList,
    chart: MetricsChart,
    metadata_bar: MetadataBar,
}

impl Default for SpvUI {
    fn default() -> Self {
        Self {
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            processes: ProcessList::new(),
            chart: MetricsChart::default(),
            metadata_bar: MetadataBar::default(),
        }
    }
}

impl SpvUI {
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>) {
        let layout = UiLayout::new(frame);

        self.metadata_bar.set_selected_process(ProcessMetadata::new(1234, "ping"));

        self.tabs.render(frame, layout.tabs_chunk());
        self.processes.render(frame, layout.processes_chunk());
        self.chart.render(frame, layout.chart_chunk());
        self.metadata_bar.render(frame, layout.metadata_chunk());
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        self.processes.set_processes(processes);
    }
}
