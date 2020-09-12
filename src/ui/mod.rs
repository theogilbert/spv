use tui::Frame;

use crate::app::TuiBackend;
use crate::ui::chart::MetricsChart;
use crate::ui::layout::UiLayout;
use crate::ui::metadata::MetadataBar;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;

// Tabs, ProcessList etc... should not leak. FrameRenderer will have next_tab() etc... methods
mod layout;
mod tabs;
mod processes;
mod chart;
mod metadata;

pub struct FrameRenderer {
    tabs: MetricTabs,
    processes: ProcessList,
    chart: MetricsChart,
    metadata_bar: MetadataBar,
}

impl Default for FrameRenderer {
    fn default() -> Self {
        Self {
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            processes: ProcessList::new(),
            chart: MetricsChart::default(),
            metadata_bar: MetadataBar::default(),
        }
    }
}

impl FrameRenderer {
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>) {
        let layout = UiLayout::new(frame);

        self.tabs.render(frame, layout.tabs_chunk());
        self.processes.render(frame, layout.processes_chunk(), &[]);
        self.chart.render(frame, layout.chart_chunk());
        self.metadata_bar.render(frame, layout.metadata_chunk());
    }
}
