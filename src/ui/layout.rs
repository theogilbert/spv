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

pub fn centered_area(parent_area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(parent_area.width);
    let height = height.min(parent_area.height);

    let remaining_width = parent_area.width - width;
    let remaining_height = parent_area.height - height;

    Rect::new(
        parent_area.left() + remaining_width / 2,
        parent_area.top() + remaining_height / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod test_centered_area {
    use tui::layout::Rect;

    use crate::ui::layout::centered_area;

    #[test]
    fn should_return_centered_area() {
        let parent_area = Rect::new(5, 5, 15, 15);
        let centered_area = centered_area(parent_area, 5, 5);

        assert_eq!(centered_area, Rect::new(10, 10, 5, 5));
    }

    #[test]
    fn should_handle_case_where_parent_area_smaller_than_given_dimensions() {
        let parent_area = Rect::new(0, 0, 10, 10);
        let centered_area = centered_area(parent_area, 20, 20);

        assert_eq!(centered_area, Rect::new(0, 0, 10, 10));
    }
}
