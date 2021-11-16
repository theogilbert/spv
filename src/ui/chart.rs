use std::time::Duration;

use tui::style::{Color, Style};
use tui::symbols;
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};

use crate::core::iteration::Iteration;
use crate::core::view::MetricView;
use crate::ui::labels::TimeLabelMaker;
use crate::ui::terminal::FrameRegion;
use crate::ui::Error;

pub struct MetricsChart {
    time_label_maker: TimeLabelMaker,
}

impl MetricsChart {
    pub fn new(resolution: Duration) -> Self {
        MetricsChart {
            time_label_maker: TimeLabelMaker::new(resolution),
        }
    }

    pub fn render(&self, frame: &mut FrameRegion, view: &MetricView, current_iter: Iteration) -> Result<(), Error> {
        let raw_data = build_raw_vecs(view);

        let chart = Chart::new(build_datasets(&raw_data, view))
            .block(Block::default().borders(Borders::ALL))
            .x_axis(self.define_x_axis(view, current_iter)?)
            .y_axis(self.define_y_axis(view));

        frame.render_widget(chart);
        Ok(())
    }

    fn define_x_axis(&self, metrics_view: &MetricView, current_iter: Iteration) -> Result<Axis, Error> {
        let (begin, end) = (metrics_view.span().begin(), metrics_view.span().end());
        let labels = vec![
            Span::from(self.time_label_maker.relative_label(current_iter, begin)?),
            Span::from(self.time_label_maker.relative_label(current_iter, end)?),
        ];

        Ok(Axis::default()
            .style(Style::default().fg(Color::White))
            .bounds([
                metrics_view.span().signed_begin() as f64,
                metrics_view.span().end() as f64,
            ])
            .labels(labels))
    }

    fn define_y_axis(&self, metrics_view: &MetricView) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * metrics_view.max_f64()).max(MINIMUM_UPPER_BOUND);

        let labels = vec![
            Span::from("0"),
            Span::from(metrics_view.concise_repr_of_value(upper_bound)),
        ];

        Axis::default()
            .title(metrics_view.unit())
            .style(Style::default().fg(Color::White))
            .bounds([0., upper_bound]) // 0 to 1.1 * max(dataset.y)
            .labels(labels)
    }
}

fn build_datasets<'a>(raw_data: &'a [Vec<(f64, f64)>], metrics_view: &MetricView) -> Vec<Dataset<'a>> {
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

            let ds_style = Style::default().fg(COLORS[index]);

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

    let first_iteration = metrics_view.first_iteration();

    for dimension_idx in 0..metrics_cardinality {
        let data: Vec<_> = metrics_view
            .as_slice()
            .iter()
            .map(|m| m.as_f64(dimension_idx).expect("Error accessing raw metric value"))
            .enumerate()
            .map(|(idx, raw_value)| ((first_iteration + idx) as f64, raw_value))
            .collect();

        data_vecs.push(data);
    }

    data_vecs
}

#[cfg(test)]
mod test_raw_data_from_metrics_view {
    use crate::core::collection::ProcessData;
    use crate::core::iteration::Span;
    use crate::core::metrics::IOMetric;
    use crate::ui::chart::build_raw_vecs;

    #[test]
    fn test_should_assign_correct_iteration_to_each_metric() {
        let mut process_data = ProcessData::<IOMetric>::new(7);
        process_data.push(IOMetric::new(10, 20));
        process_data.push(IOMetric::new(30, 40));

        let metrics_view = process_data.view(Span::new(0, 10));
        let raw_vecs = build_raw_vecs(&metrics_view);

        assert_eq!(
            raw_vecs,
            vec![vec![(7.0, 10.0), (8.0, 30.0)], vec![(7.0, 20.0), (8.0, 40.0)]]
        );
    }
}
