use std::time::Duration;

use log::error;
use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::app::TuiBackend;
use crate::core::metrics::Archive;
use crate::core::process_view::ProcessMetadata;

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
    fn build_process_dataset<'a>(&self, process: &'a ProcessMetadata, metrics: &'a Archive) -> Dataset<'a> {
        // (0..self.span.as_secs())
        //     .map(|i| (i as f64))
        //     .map(|i| (i as f64, i.cos()))
        //     .collect();

        Dataset::default()
            .name(process.command())
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
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
            let dataset = vec![self.build_process_dataset(process, metrics)];

            let chart = Chart::new(dataset)
                .block(Block::default()
                    .borders(Borders::ALL))
                .x_axis(Axis::default()
                    .style(Style::default().fg(Color::White))
                    .bounds([0.0, 10.])// min(dataset.x) to max(dataset.x)
                    .labels([&self.axis_origin_label, "0"].iter().cloned().map(Span::from).collect()))
                .y_axis(Axis::default()
                    .title(self.get_label_unit(metrics))
                    .style(Style::default().fg(Color::White))
                    .bounds([-2., 2.]) // 0 to max(dataset.y)
                    .labels(["-2.0", "0.0", "2.0"].iter().cloned().map(Span::from).collect()));

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