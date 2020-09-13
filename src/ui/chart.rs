use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::app::TuiBackend;

pub struct MetricsChart;

impl Default for MetricsChart {
    fn default() -> Self {
        Self {}
    }
}

// TODO specify resolution in seconds (e.g. resolution=1 means 1 second <-> 1 character)
impl MetricsChart {
    fn generate_data(&self) -> Vec<(f64, f64)> {
        (0..1000)
            .map(|i| (i as f64) * 0.01)
            .map(|i| (i, i.cos()))
            .collect()
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect) {
        let data = self.generate_data();

        let dataset = vec![
            Dataset::default()
                .name("ping")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .data(&data),
        ];

        let chart = Chart::new(dataset)
            .block(Block::default()
                .borders(Borders::ALL))
            .x_axis(Axis::default()
                .style(Style::default().fg(Color::White))
                .bounds([0.0, 10.])// min(dataset.x) to max(dataset.x)
                .labels([""].iter().cloned().map(Span::from).collect()))
            .y_axis(Axis::default()
                .title("%")
                .style(Style::default().fg(Color::White))
                .bounds([-2., 2.]) // 0 to max(dataset.y)
                .labels(["-2.0", "0.0", "2.0"].iter().cloned().map(Span::from).collect()));

        frame.render_widget(chart, chunk);
    }
}