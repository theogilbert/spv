use tui::style::{Color, Style};
use tui::symbols;
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::view::MetricView;
use crate::ui::terminal::FrameRegion;

pub struct MetricsChart {}

impl Default for MetricsChart {
    fn default() -> Self {
        MetricsChart {}
    }
}

impl MetricsChart {
    pub fn render(&self, frame: &mut FrameRegion, metrics_view: &MetricView) {
        let data_frame = DataFrame::new(metrics_view);

        let chart = Chart::new(data_frame.datasets())
            .block(Block::default().borders(Borders::ALL))
            .x_axis(self.define_x_axis(metrics_view))
            .y_axis(self.define_y_axis(&data_frame, metrics_view.max_concise_repr(), metrics_view.unit()));

        frame.render_widget(chart);
    }

    fn define_x_axis(&self, metrics_view: &MetricView) -> Axis {
        let labels = ["-1m", "now"] // TODO make this actually reflect metrics_view
            .iter()
            .cloned()
            .map(Span::from)
            .collect();

        Axis::default()
            .style(Style::default().fg(Color::White))
            .bounds([
                // We do not use span.begin() here, as it has a min value of 0. On application startup, the bounds
                // would be squashed between 0 and span.end() until span.end() would be greater than span.size()
                metrics_view.span().end() as f64 - metrics_view.span().size() as f64,
                metrics_view.span().end() as f64,
            ])
            .labels(labels)
    }

    fn define_y_axis(&self, data_frame: &DataFrame, upper_bound_repr: String, unit: &'static str) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * data_frame.max_value()).max(MINIMUM_UPPER_BOUND);

        Axis::default()
            .title(unit)
            .style(Style::default().fg(Color::White))
            .bounds([0., upper_bound]) // 0 to 1.1 * max(dataset.y)
            .labels(MetricsChart::build_y_axis_labels(upper_bound_repr))
    }

    fn build_y_axis_labels<'a>(upper_bound_repr: String) -> Vec<Span<'a>> {
        vec![Span::from("0"), Span::from(upper_bound_repr)]
    }
}

// TODO refactor - DataFrame does not need to be an object. Replace it by functions / and test them
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
            data: Self::extract_raw_from_metrics(metrics_view),
        }
    }

    /// Returns datasets built from the metric view
    /// Each dataset in the returned `Vec` corresponds to one dimension of the metrics
    pub fn datasets(&self) -> Vec<Dataset> {
        const COLORS: [Color; 2] = [Color::Blue, Color::Green];

        self.data
            .iter()
            .enumerate()
            .map(|(index, data)| {
                let name = self
                    .metrics_view
                    .last_or_default()
                    .explicit_repr(index)
                    // panic should never happen as index should never be greater than cardinality:
                    .expect("Invalid index when building dataframe");

                let ds_style = Style::default().fg(COLORS[index % COLORS.len()]);

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
    fn extract_raw_from_metrics(metrics_view: &MetricView) -> Vec<Vec<(f64, f64)>> {
        let mut data_vecs: Vec<_> = Vec::new();
        let metrics_cardinality = metrics_view.last_or_default().cardinality();

        let last_iter = metrics_view.last_iteration();

        for dimension_idx in 0..metrics_cardinality {
            let data: Vec<_> = metrics_view
                .as_slice()
                .iter()
                .rev()
                .map(|m| m.as_f64(dimension_idx).expect("Error accessing raw metric value"))
                .enumerate()
                .map(|(idx, raw_value)| ((last_iter - idx) as f64, raw_value))
                .rev()
                .collect();

            data_vecs.push(data);
        }

        data_vecs
    }

    pub fn max_value(&self) -> f64 {
        self.metrics_view.max_f64()
    }
}
