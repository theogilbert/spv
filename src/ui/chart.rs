use std::cmp::Ordering;
use std::result::Iter;
use std::time::Duration;

use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::metrics::{Archive, Metric};
use crate::core::process_view::{PID, ProcessMetadata};
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
            axis_origin_label: "-1m".to_string(), // TODO make this actually reflect self.span
        }
    }

    fn metrics_history<'a>(&self, archive: &'a Archive, process: &ProcessMetadata, current_label: &str) -> &'a [Metric] {
        archive.history(current_label, process.pid(), self.span)
            .expect("Error getting history of process")
    }

    fn get_label_unit(&self, archive: &Archive, current_label: &str) -> &'static str {
        archive.label_unit(current_label)
            .expect("Error while getting label unit")
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect,
                  process_opt: Option<&ProcessMetadata>, archive: &Archive, current_label: &str) {
        if let Some(process) = process_opt {
            let history = self.metrics_history(archive, process, current_label);
            let data_frame = DataFrame::new(history,
                                            archive.default_metric(current_label).unwrap(),
                                            archive.step());

            let max_repr = data_frame.max_repr();

            let chart = Chart::new(data_frame.datasets())
                .block(Block::default()
                    .borders(Borders::ALL))
                .x_axis(Axis::default()
                    .style(Style::default().fg(Color::White))
                    .bounds([0. - self.span.as_secs_f64(), 0.0])
                    .labels([&self.axis_origin_label, "-0m"].iter().cloned().map(Span::from).collect()))
                .y_axis(Axis::default()
                    .title(self.get_label_unit(archive, current_label))
                    .style(Style::default().fg(Color::White))
                    .bounds([0., 1.1 * data_frame.max_value()]) // 0 to 1.1 * max(dataset.y)
                    .labels(["0.0", &max_repr].iter().cloned().map(Span::from).collect()));

            frame.render_widget(chart, chunk);
        }
    }
}


/// Performs all required operations to get raw "drawable" data from `&[&Metric]`
struct DataFrame<'a> {
    metrics: &'a [Metric],
    default: Metric,
    // Nested vec to support multi-dimensional metrics. Each item from the outer Vec is a dimension
    data: Vec<Vec<(f64, f64)>>,
}

impl<'a> DataFrame<'a> {
    pub fn new(metrics: &'a [Metric], default: Metric, step: Duration) -> Self {
        let data = Self::extract_raw_from_metrics(metrics, step);

        Self { metrics, data, default }
    }

    /// Extract raw data from a collection of metrics
    /// Raw data consists of sets of (f64, f64) tuples, each set corresponding to a drawable
    /// `Dataset`
    fn extract_raw_from_metrics(metrics: &'a [Metric], step: Duration) -> Vec<Vec<(f64, f64)>> {
        let mut data_vecs: Vec<_> = Vec::new();

        if let Some(first) = metrics.first() {
            for i in 0..first.cardinality() {
                let data: Vec<_> = metrics.iter()
                    .rev()
                    .map(|m| {
                        m.raw_as_f64(i)
                            .expect("Error accessing raw metric value")
                    })
                    .enumerate()
                    .map(|(t, r)| (0. - (t as f64 * step.as_secs_f64()), r))
                    .collect();

                data_vecs.push(data);
            }
        }

        data_vecs
    }

    /// Returns datasets built from the [`Metric`](enum.Metric) instances
    /// Each element in the returned `Vec` corresponds to one dimension
    pub fn datasets(&self) -> Vec<Dataset> {
        const COLORS: [Color; 2] = [Color::Blue, Color::Green];

        self.data.iter()
            .enumerate()
            .map(|(index, data)| {
                let name = self.metrics.last()
                    .unwrap_or(&&self.default) // fails if self.metrics is empty
                    .explicit_repr(index)
                    // panic should never happen as index should never be greater than cardinality:
                    .unwrap();

                let ds_style = Style::default()
                    .fg(COLORS[index % COLORS.len()]);

                Dataset::default()
                    .name(name)
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(ds_style)
                    .data(data)
            })
            .collect()
    }

    fn max_metric(&self) -> &Metric {
        self.metrics.iter()
            .max_by(|m1, m2| m1.partial_cmp(m2).unwrap_or(Ordering::Equal))
            .unwrap_or(&&self.default)
    }

    pub fn max_value(&self) -> f64 {
        self.max_metric()
            .raw_iter()
            .max_by(|m1, m2| m1.partial_cmp(m2).unwrap_or(Ordering::Equal))
            .unwrap()
    }

    pub fn max_repr(&self) -> String {
        self.max_metric()
            .concise_repr()
    }
}


#[cfg(test)]
mod test_data_frame {
    use std::time::Duration;

    use crate::core::metrics::Metric;
    use crate::ui::chart::DataFrame;

    #[test]
    fn test_max_metric() {
        let metrics = vec![
            Metric::IO { input: 20, output: 40 },
            Metric::IO { input: 5, output: 50 }
        ];
        let df = DataFrame::new(&metrics,
                                Metric::IO { input: 0, output: 0 },
                                Duration::from_secs(1));

        assert_eq!(df.max_metric(), &metrics[1]);
    }

    #[test]
    fn test_max_value() {
        let df = DataFrame::new(&[Metric::IO { input: 20, output: 50 }],
                                Metric::IO { input: 0, output: 0 },
                                Duration::from_secs(1));

        assert_eq!(df.max_value(), 50.);
    }
}