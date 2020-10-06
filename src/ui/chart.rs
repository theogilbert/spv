use std::time::Duration;

use log::error;
use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::metrics::Archive;
use crate::core::process_view::ProcessMetadata;
use crate::ui::terminal::TuiBackend;

pub struct MetricsChart {
    // The time span the chart covers
    span: Duration,
    axis_origin_label: String,
    current_label: String,
}

impl Default for MetricsChart {
    fn default() -> Self {
        Self {
            span: Duration::from_secs(60),
            axis_origin_label: "-1m".to_string(),
            current_label: "CPU Usage".to_string(),
        }
    }
}


impl MetricsChart {
    fn build_process_data(&self, process: &ProcessMetadata, metrics: &Archive) -> Vec<(f64, f64)> {
        metrics.history(&self.current_label, process.pid(), self.span)
            .expect("Could not get history of process")
            .rev()
            .enumerate()
            .map(|(i, m)| (0. - i as f64, m.as_f64()))
            .collect()
    }

    fn get_label_unit(&self, metrics: &Archive) -> &'static str {
        metrics.label_unit(&self.current_label)
            .map_err(|e| {
                error!("Error while getting label unit");
                e
            })
            .expect("Internal error")
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect,
                  process_opt: Option<&ProcessMetadata>, metrics: &Archive) {
        if let Some(process) = process_opt {
            let data = self.build_process_data(process, metrics);

            let max = data.iter()
                .map(|(_, v)| v.ceil() as u32)
                .max()
                .unwrap_or(0) as f64;
            let max_repr = max.to_string();

            let datasets = vec![
                Dataset::default()
                    .name(process.command())
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .data(&data)];

            let chart = Chart::new(datasets)
                .block(Block::default()
                    .borders(Borders::ALL))
                .x_axis(Axis::default()
                    .style(Style::default().fg(Color::White))
                    .bounds([0. - metrics.expected_metrics(self.span) as f64, 0.0])// min(dataset.x) to max(dataset.x)
                    .labels([&self.axis_origin_label, "-0m"].iter().cloned().map(Span::from).collect()))
                .y_axis(Axis::default()
                    .title(self.get_label_unit(metrics))
                    .style(Style::default().fg(Color::White))
                    .bounds([0., 1.1 * max]) // 0 to 1.1 * max(dataset.y)
                    .labels(["0.0", &max_repr].iter().cloned().map(Span::from).collect()));

            frame.render_widget(chart, chunk);
        }
    }
}

// #[cfg(test)]
// mod test_metrics_chart {
//     #[fixture]
//     fn cosinus_data() -> Vec<(f64, f64)> {
//         (0..1000)
//             .map(|i| (i as f64) * 0.01)
//             .map(|i| (i, i.cos()))
//             .collect()
//     }
// }