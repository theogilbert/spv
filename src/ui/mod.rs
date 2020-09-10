use std::io;
use std::io::Stdout;

use termion::raw::{IntoRawMode, RawTerminal};
use tui::{Frame, Terminal};
use tui::backend::TermionBackend;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::Spans;
use tui::widgets::{Block, Borders, BorderType, List, ListItem, ListState, StatefulWidget, Tabs, Widget};

use crate::app::TuiBackend;
use crate::probe::MetricSet;
use crate::probe::process::{PID, ProcessMetadata};
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

impl FrameRenderer {
    pub fn new() -> Self {
        Self {
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()]),
            processes: ProcessList::new(),
        }
    }

    pub fn render(&mut self, frame: &mut Frame<TuiBackend>) {
        let layout = UiLayout::new(frame);

        frame.render_widget(self.tabs.refreshed_tabs(), layout.tabs_chunk());

        let (processes_widget, mut processes_state) = self.processes.refreshed_list(&[]);

        frame.render_stateful_widget(processes_widget, layout.processes_chunk(), &mut processes_state);
    }
}
