use tui::Frame;

use crate::app::TuiBackend;
use crate::ui::layout::UiLayout;
use crate::ui::processes::ProcessList;
use crate::ui::tabs::MetricTabs;

// Tabs, ProcessList etc... should not leak. FrameRenderer will have next_tab() etc... methods
mod layout;
mod tabs;
mod processes;

pub struct FrameRenderer {
    tabs: MetricTabs,
    processes: ProcessList,
}

impl Default for FrameRenderer {
    fn default() -> Self {
        Self {
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            processes: ProcessList::new(),
        }
    }
}

impl FrameRenderer {
    pub fn render(&mut self, frame: &mut Frame<TuiBackend>) {
        let layout = UiLayout::new(frame);

        frame.render_widget(self.tabs.refreshed_tabs(), layout.tabs_chunk());

        let (processes_widget, mut processes_state) = self.processes.refreshed_list(&[]);

        frame.render_stateful_widget(processes_widget, layout.processes_chunk(), &mut processes_state);
    }
}
