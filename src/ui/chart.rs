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
        let raw_data = build_raw_vecs(metrics_view);

        let chart = Chart::new(build_datasets(&raw_data, metrics_view))
            .block(Block::default().borders(Borders::ALL))
            .x_axis(self.define_x_axis(metrics_view))
            .y_axis(self.define_y_axis(metrics_view, metrics_view.max_concise_repr(), metrics_view.unit()));

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

    fn define_y_axis(&self, metrics_view: &MetricView, upper_bound_repr: String, unit: &'static str) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * metrics_view.max_f64()).max(MINIMUM_UPPER_BOUND);

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

fn build_datasets<'a>(raw_data: &'a Vec<Vec<(f64, f64)>>, metrics_view: &MetricView) -> Vec<Dataset<'a>> {
    const COLORS: [Color; 2] = [Color::Blue, Color::Green];

    raw_data
        .iter()
        .enumerate()
        .map(|(index, data)| {
            let name = metrics_view
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

fn build_raw_vecs(metrics_view: &MetricView) -> Vec<Vec<(f64, f64)>> {
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

#[cfg(test)]
mod test_raw_data_from_metrics_view {
    use crate::core::iteration::{Iteration, Span};
    use crate::core::metrics::{IOMetric, Metric};
    use crate::core::view::MetricView;
    use crate::ui::chart::build_raw_vecs;

    #[test]
    fn test_should_assign_correct_iteration_to_each_metric() {
        let metrics_data = vec![IOMetric::new(10, 20), IOMetric::new(30, 40)];
        let default = IOMetric::default();

        let dyn_metrics_vec = metrics_data.iter().map(|m| m as &dyn Metric).collect();
        let metrics_view = MetricView::new(dyn_metrics_vec, &default, Span::from_end_and_size(10, 10), 8);
        let raw_vecs = build_raw_vecs(&metrics_view);

        let expected_raw_vecs = vec![vec![(7.0, 10.0), (8.0, 30.0)], vec![(7.0, 20.0), (8.0, 40.0)]];

        assert_eq!(raw_vecs, expected_raw_vecs);
    }
}
