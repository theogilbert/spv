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

        frame.render_widget(self.tabs.refreshed_tabs(), layout.tabs_chunk());

        let (processes_widget, mut processes_state) = self.processes.refreshed_list(&[]);

        frame.render_stateful_widget(processes_widget, layout.processes_chunk(), &mut processes_state);


        frame.render_widget(self.chart.refreshed_chart("ping", &self.chart.generate_data()),
                            layout.chart_chunk());

        frame.render_widget(self.metadata_bar.refreshed_metadata(), layout.metadata_chunk());
    }
}
