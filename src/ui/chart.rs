use tui::style::{Color, Style};
use tui::symbols;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};
use tui::text::Span;

pub struct MetricsChart;

impl Default for MetricsChart {
    fn default() -> Self {
        Self {}
    }
}

impl MetricsChart {
    pub fn generate_data(&self) -> Vec<(f64, f64)> {
        (0..1000)
            .map(|i| (i as f64) * 0.01)
            .map(|i| (i, i.cos()))
            .collect()
    }

    pub fn refreshed_chart<'a>(&self, cmd: &'a str, data: &'a [(f64, f64)]) -> Chart<'a> {
        let dataset = vec![
            Dataset::default()
                .name(cmd)
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .data(data),
        ];

        Chart::new(dataset)
            .block(Block::default()
                .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
            .x_axis(Axis::default()
                .title("seconds")
                .style(Style::default().fg(Color::White))
                .bounds([0.0, 10.])// min(dataset.x) to max(dataset.x)
                .labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()))
            .y_axis(Axis::default()
                .title("%")
                .style(Style::default().fg(Color::White))
                .bounds([-2., 2.]) // 0 to max(dataset.y)
                .labels(["-2.0", "0.0", "2.0"].iter().cloned().map(Span::from).collect()))
    }
}