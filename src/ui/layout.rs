use tui::Frame;
use tui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::TuiBackend;

pub struct UiLayout {
    main_chunks: Vec<Rect>,
    center_chunks: Vec<Rect>,
}

impl UiLayout {
    pub fn new(frame: &Frame<TuiBackend>) -> Self {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Percentage(100),
                    Constraint::Length(1),
                ].as_ref()
            )
            .split(frame.size());

        let center_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(10),
                    Constraint::Percentage(100),
                ].as_ref()
            )
            .split(*main_chunks.get(1).unwrap());

        Self { main_chunks, center_chunks }
    }

    pub fn tabs_chunk(&self) -> Rect {
        self.main_chunks[0]
    }

    pub fn processes_chunk(&self) -> Rect {
        self.center_chunks[0]
    }

    pub fn graph_chunk(&self) -> Rect {
        self.center_chunks[1]
    }

    pub fn metadata_chunk(&self) -> Rect {
        self.main_chunks[2]
    }
}