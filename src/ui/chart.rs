use std::cmp::Ordering;
use std::time::Duration;

use tui::{Frame, symbols};
use tui::layout::Rect;
use tui::style::{Color, Style};
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::view::MetricView;
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

    pub fn render(&self, frame: &mut Frame<TuiBackend>, chunk: Rect, metrics_view: &MetricView) {
        let data_frame = DataFrame::new(metrics_view);

        let chart = Chart::new(data_frame.datasets())
            .block(Block::default().borders(Borders::ALL))
            .x_axis(self.define_x_axis())
            .y_axis(self.define_y_axis(&data_frame, metrics_view.unit()));

        frame.render_widget(chart, chunk);
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

    fn define_y_axis(&self, data_frame: &DataFrame, unit: &'static str) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * data_frame.max_value()).max(MINIMUM_UPPER_BOUND);

        Axis::default()
            .title(unit)
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
}


/// Performs all required operations to get raw "drawable" data from `&[&Metric]`
struct DataFrame<'a> {
    metrics_view: &'a MetricView<'a>,
    // data has to be persisted as an attr, to be able to return a Dataset which references data
    // from this vec
    data: Vec<Vec<(f64, f64)>>,
}

impl<'a> DataFrame<'a> {
    pub fn new(metrics_view: &'a MetricView) -> Self {
        Self {
            metrics_view,
            data: Self::extract_raw_from_metrics(metrics_view, metrics_view.resolution()),
        }
    }

    /// Returns datasets built from the [`Metric`](enum.Metric) instances
    /// Each element in the returned `Vec` corresponds to one dimension
    pub fn datasets(&self) -> Vec<Dataset> {
        const COLORS: [Color; 2] = [Color::Blue, Color::Green];

        self.data.iter()
            .enumerate()
            .map(|(index, data)| {
                let name = self.metrics_view.last_or_default()
                    .explicit_repr(index)
                    // panic should never happen as index should never be greater than cardinality:
                    .expect("Invalid index when building dataframe");

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


    /// Extract raw data from a collection of metrics
    /// Raw data consists of sets of (f64, f64) tuples, each set corresponding to a drawable
    /// `Dataset`
    fn extract_raw_from_metrics(metrics_view: &MetricView, step: Duration) -> Vec<Vec<(f64, f64)>> {
        let mut data_vecs: Vec<_> = Vec::new();
        let metrics_cardinality = metrics_view
            .last_or_default()
            .cardinality();

        for dimension_idx in 0..metrics_cardinality {
            let data: Vec<_> = metrics_view.as_slice()
                .iter()
                .rev()
                .map(|m| {
                    m.as_f64(dimension_idx)
                        .expect("Error accessing raw metric value")
                })
                .enumerate()
                .map(|(t, r)| (0. - (t as f64 * step.as_secs_f64()), r))
                .collect();

            data_vecs.push(data);
        }

        data_vecs
    }

    pub fn max_value(&self) -> f64 {
        self.metrics_view.as_slice()
            .iter()
            .map(|m| m.max_value())
            .max_by(|f1, f2| f1.partial_cmp(f2).unwrap_or(Ordering::Equal))
            .unwrap_or(0.)
    }
}

//
// #[cfg(test)]
// mod test_data_frame {
//     use std::time::Duration;
//
//     use crate::core::metrics::Metric;
//     use crate::ui::chart::DataFrame;
//
//     #[test]
//     fn test_max_metric() {
//         let metrics = vec![
//             Metric::IO { input: 20, output: 40 },
//             Metric::IO { input: 5, output: 50 },
//         ];
//         let df = DataFrame::new(&metrics,
//                                 Metric::IO { input: 0, output: 0 },
//                                 Duration::from_secs(1));
//
//         assert_eq!(df.max_metric(), &metrics[1]);
//     }
//
//     #[test]
//     fn test_max_value() {
//         let df = DataFrame::new(&[Metric::IO { input: 20, output: 50 }],
//                                 Metric::IO { input: 0, output: 0 },
//                                 Duration::from_secs(1));
//
//         assert_eq!(df.max_value(), 50.);
//     }
// }