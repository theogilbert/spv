use std::cmp::Ordering;
use std::time::Duration;

use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::metrics::{Archive, Metric};
use crate::core::process_view::ProcessMetadata;
use crate::ui::terminal::TuiBackend;

pub struct MetricsChart {
    // The time span the chart covers
    span: Duration,
    axis_origin_label: String,
}


// TODO Keep displaying "dead" processes if they are covered by `span`
impl MetricsChart {
    pub fn new(span: Duration) -> Self {
        Self {
            span,
            axis_origin_label: "-1m".to_string(), // TODO make this actually reflect self.span
        }
    }

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect,
                  process: &ProcessMetadata, archive: &Archive, current_label: &str) {
        let history = self.metrics_history(archive, process, current_label);
        let data_frame = DataFrame::new(history,
                                        archive.default_metric(current_label).unwrap(),
                                        archive.step());

        let chart = Chart::new(data_frame.datasets())
            .block(Block::default().borders(Borders::ALL))
            .x_axis(self.define_x_axis())
            .y_axis(self.define_y_axis(archive, &data_frame, current_label));

        frame.render_widget(chart, chunk);
    }

    fn metrics_history<'a>(&self, archive: &'a Archive, process: &ProcessMetadata, current_label: &str) -> &'a [Metric] {
        archive.history(current_label, process.pid(), self.span)
            .expect("Error getting history of process")
    }

    fn define_x_axis(&self) -> Axis {
        let labels = [&self.axis_origin_label, "-0m"].iter()
            .cloned()
            .map(Span::from)
            .collect();

        Axis::default()
            .style(Style::default().fg(Color::White))
            .bounds([0. - self.span.as_secs_f64(), 0.0])
            .labels(labels)
    }

    fn define_y_axis(&self, archive: &Archive, data_frame: &DataFrame, current_label: &str) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * data_frame.max_value()).max(MINIMUM_UPPER_BOUND);

        Axis::default()
            .title(self.get_label_unit(archive, current_label))
            .style(Style::default().fg(Color::White))
            .bounds([0., upper_bound]) // 0 to 1.1 * max(dataset.y)
            .labels(MetricsChart::build_y_axis_labels(upper_bound))
    }

    fn build_y_axis_labels<'a>(upper_bound: f64) -> Vec<Span<'a>> {
        return vec! {
            Span::from("0.0"),
            Span::from((upper_bound as i32).to_string())
        };
    }

    fn get_label_unit(&self, archive: &Archive, current_label: &str) -> &'static str {
        archive.label_unit(current_label)
            .expect("Error while getting label unit")
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

        Self { metrics, default, data }
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
                    .unwrap_or(&self.default) // fails if self.metrics is empty
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
            .unwrap_or(&self.default)
    }

    pub fn max_value(&self) -> f64 {
        self.max_metric()
            .raw_iter()
            .max_by(|m1, m2| m1.partial_cmp(m2).unwrap_or(Ordering::Equal))
            .unwrap()
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
            Metric::IO { input: 5, output: 50 },
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