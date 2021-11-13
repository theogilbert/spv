use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct UiLayout {
    main_chunks: Vec<Rect>,
    center_chunks: Vec<Rect>,
}

impl UiLayout {
    pub fn new(region: Rect) -> Self {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1), // tabs constraint
                    Constraint::Min(1),    // center region constraint
                    Constraint::Length(1), // metadata constraint
                ]
                .as_ref(),
            )
            .split(region);

        let center_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(30), // Processes constraint
                    Constraint::Min(1),     // graph constraint
                ]
                .as_ref(),
            )
            .split(*main_chunks.get(1).unwrap());

        Self {
            main_chunks,
            center_chunks,
        }
    }

    pub fn tabs_chunk(&self) -> Rect {
        self.main_chunks[0]
    }

    pub fn processes_chunk(&self) -> Rect {
        self.center_chunks[0]
    }

    pub fn chart_chunk(&self) -> Rect {
        self.center_chunks[1]
    }

    pub fn metadata_chunk(&self) -> Rect {
        self.main_chunks[2]
    }
}
