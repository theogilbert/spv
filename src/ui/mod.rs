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
use crate::ui::tabs::MetricTabs;

pub mod layout;
pub mod tabs;

pub struct FrameRenderer {
    tabs: MetricTabs,
}

impl FrameRenderer {
    pub fn new() -> Self {
        Self {
            tabs: MetricTabs::new(vec!["CPU Usage".to_string()])
        }
    }

    pub fn render(&mut self, frame: &mut Frame<TuiBackend>) {
        let layout = UiLayout::new(frame);

        frame.render_widget(self.tabs.refreshed_tabs(), layout.tabs_chunk());
    }
}
