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
}


impl MetricsChart {
    pub fn new(span: Duration) -> Self {
        Self {
            span,
            axis_origin_label: "-1m".to_string(), // TODO fix this
        }
    }

    fn collect_data(&self, process: &ProcessMetadata, metrics: &Archive, current_label: &str) -> Vec<Vec<(f64, f64)>> {
        let default = metrics.default_metric(current_label)
            .expect("Could not get default metric");
        let mut data_vecs = Vec::new();

        for i in 0..default.cardinality() {
            data_vecs.push(metrics.history(current_label, process.pid(), self.span)
                .expect("Could not get history of process")
                .rev()
                .map(|m| {
                    m.raw(i)
                        .expect("Could not access raw metric value")
                })
                .enumerate()
                .map(|(t, r)| (0. - t as f64, r))
                .collect());
        }

        data_vecs
    }

    fn build_datasets<'a>(process: &'a ProcessMetadata, data: &'a Vec<Vec<(f64, f64)>>) -> Vec<Dataset<'a>> {
        data.iter()
            .map(|d| {
                Dataset::default()
                    .name(process.command())
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .data(d)
            })
            .collect()
    }

    fn get_label_unit(&self, metrics: &Archive, current_label: &str) -> &'static str {
        metrics.label_unit(current_label)
            .map_err(|e| {
                error!("Error while getting label unit");
                e
            })
            .expect("Internal error")
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect,
                  process_opt: Option<&ProcessMetadata>, metrics: &Archive, current_label: &str) {
        if let Some(process) = process_opt {
            let data = self.collect_data(process, metrics, current_label);
            let datasets = Self::build_datasets(process, &data);
            let max = Self::retrieve_max_value_from_data_vec(&data);
            let max_repr = max.to_string();

            let chart = Chart::new(datasets)
                .block(Block::default()
                    .borders(Borders::ALL))
                .x_axis(Axis::default()
                    .style(Style::default().fg(Color::White))
                    .bounds([0. - metrics.expected_metrics(self.span) as f64, 0.0])// min(dataset.x) to max(dataset.x)
                    .labels([&self.axis_origin_label, "-0m"].iter().cloned().map(Span::from).collect()))
                .y_axis(Axis::default()
                    .title(self.get_label_unit(metrics, current_label))
                    .style(Style::default().fg(Color::White))
                    .bounds([0., 1.1 * max]) // 0 to 1.1 * max(dataset.y)
                    .labels(["0.0", &max_repr].iter().cloned().map(Span::from).collect()));

            frame.render_widget(chart, chunk);
        }
    }

    fn retrieve_max_value_from_data_vec(data: &Vec<Vec<(f64, f64)>>) -> f64 {
        data.iter()
            .map(|d| {
                // We get the max value of each sub-vec of data
                d.iter()
                    .map(|(_, v)| v.ceil() as u32)
                    .max()
                    .unwrap_or(0)
            })
            .max()// and then the max value among all these sub-vec max values
            .unwrap_or(0) as f64
    }
}

#[cfg(test)]
mod test_metrics_chart {
    use std::time::Duration;

    use crate::core::metrics::{Archive, ArchiveBuilder, Metric};
    use crate::core::process_view::ProcessMetadata;
    use crate::ui::chart::MetricsChart;

    #[test]
    fn test_retrieve_max_value_from_data_vec() {
        let data: Vec<Vec<(f64, f64)>> = vec![
            vec![(0., 1.), (1., 2.), (2., 3.)],
            vec![(0., 10.), (1., 5.), (3., 7.)]];

        assert_eq!(MetricsChart::retrieve_max_value_from_data_vec(&data), 10.);
    }

    #[test]
    fn test_should_collect_all_data_with_higher_cardinality() {
        let pm = ProcessMetadata::new(1, "cmd");
        let mut metrics = ArchiveBuilder::new()
            .resolution(Duration::from_secs(1))
            .new_metric("TestMetric".to_string(), Metric::IO(0, 0))
            .expect("Could not add TestMetric")
            .build();

        metrics.push("TestMetric", 1, Metric::IO(10, 20)).unwrap();
        metrics.push("TestMetric", 1, Metric::IO(30, 40)).unwrap();

        let chart = MetricsChart::new(Duration::from_secs(2));

        assert_eq!(chart.collect_data(&pm, &metrics, "TestMetric"), vec![
            vec![(0., 30.), (-1., 10.)],
            vec![(0., 40.), (-1., 20.)]
        ]);
    }
}