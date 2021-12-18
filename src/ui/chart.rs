use std::ops::Neg;
use std::time::Duration;

use tui::layout::Alignment;
use tui::style::{Color, Style};
use tui::symbols;
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph};

use crate::core::time::Timestamp;
use crate::core::view::MetricView;
use crate::ui::labels::relative_timestamp_label;
use crate::ui::terminal::FrameRegion;

pub struct MetricsChart {
    resolution: Milliseconds,
}

impl MetricsChart {
    pub fn new(resolution: Duration) -> Self {
        Self {
            resolution: resolution.as_millis().max(1),
        }
    }

    pub fn render(&self, frame: &mut FrameRegion, view_opt: Option<&MetricView>) {
        match view_opt {
            Some(view) => self.render_metrics_view(frame, view),
            None => self.render_no_process_selected_message(frame),
        }
    }

    fn render_no_process_selected_message(&self, frame: &mut FrameRegion) {
        let block = Self::widget_block();

        let y_offset = block.inner(frame.region()).height as usize / 2;
        let y_offset = y_offset.saturating_sub(1);

        let mut text = "\n".repeat(y_offset);
        text.push_str("No process is selected");

        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center);

        frame.render_widget(paragraph)
    }

    fn render_metrics_view(&self, frame: &mut FrameRegion, view: &MetricView) {
        let raw_data = build_raw_vecs(view, self.resolution);

        let chart = Chart::new(build_datasets(&raw_data, view))
            .block(Self::widget_block())
            .x_axis(self.define_x_axis(view))
            .y_axis(self.define_y_axis(view));

        frame.render_widget(chart);
    }

    fn widget_block<'a>() -> Block<'a> {
        Block::default().borders(Borders::ALL)
    }

    fn define_x_axis(&self, metrics_view: &MetricView) -> Axis {
        let (begin, end) = (metrics_view.span().begin(), metrics_view.span().end());
        let labels = vec![
            Span::from(relative_timestamp_label(begin)),
            Span::from(relative_timestamp_label(end)),
        ];

        Axis::default()
            .style(Style::default().fg(Color::White))
            .bounds([
                calculate_x_value_of_timestamp(metrics_view.span().begin(), self.resolution),
                calculate_x_value_of_timestamp(metrics_view.span().end(), self.resolution),
            ])
            .labels(labels)
    }

    fn define_y_axis(&self, metrics_view: &MetricView) -> Axis {
        const MINIMUM_UPPER_BOUND: f64 = 10.;
        let upper_bound = (1.1 * metrics_view.max_f64()).max(MINIMUM_UPPER_BOUND);

        let labels = vec![
            Span::from("0"),
            // To keep the first X-axis label from resizing the chart when its length changes:
            // TODO remove this once issue #567 from tui-rs is solved (or PR #568 is merged)
            Span::from("           "),
            Span::from(metrics_view.concise_repr_of_value(upper_bound)),
        ];

        Axis::default()
            .title(metrics_view.unit())
            .style(Style::default().fg(Color::White))
            .bounds([0., upper_bound]) // 0 to 1.1 * max(dataset.y)
            .labels(labels)
    }
}

type Milliseconds = u128;

fn calculate_x_value_of_timestamp(timestamp: Timestamp, resolution: Milliseconds) -> f64 {
    let millis_delta = Timestamp::now().duration_since(&timestamp).as_millis();
    ((millis_delta / resolution) as f64).neg()
}

#[cfg(test)]
mod test_x_value_calculation {
    use std::time::Duration;

    use rand::random;

    use crate::core::time::test_utils::setup_fake_clock_to_prevent_substract_overflow;
    use crate::core::time::Timestamp;
    use crate::ui::chart::calculate_x_value_of_timestamp;

    #[test]
    fn test_timestamp_in_past_should_have_lower_x_value() {
        setup_fake_clock_to_prevent_substract_overflow();
        let anchor_timestamp = Timestamp::now() - Duration::from_secs(random::<u8>() as u64);
        let older_timestamp = anchor_timestamp - Duration::from_millis(2);

        let anchor_x_value = calculate_x_value_of_timestamp(anchor_timestamp, 1);
        let older_ts_x_value = calculate_x_value_of_timestamp(older_timestamp, 1);

        assert!(older_ts_x_value < anchor_x_value);
    }

    #[test]
    fn test_identical_timestamp_should_have_same_x_value() {
        setup_fake_clock_to_prevent_substract_overflow();
        let anchor_timestamp = Timestamp::now() - Duration::from_secs(random::<u8>() as u64);

        let x_value_1 = calculate_x_value_of_timestamp(anchor_timestamp, 1);
        let x_value_2 = calculate_x_value_of_timestamp(anchor_timestamp, 1);

        assert_eq!(x_value_1, x_value_2);
    }

    #[test]
    fn test_timestamps_within_same_resolution_should_have_same_x_value() {
        setup_fake_clock_to_prevent_substract_overflow();

        let anchor_timestamp = Timestamp::now();
        let similar_timestamp = Timestamp::now() - Duration::from_millis(49);

        let x_value_1 = calculate_x_value_of_timestamp(anchor_timestamp, 50);
        let x_value_2 = calculate_x_value_of_timestamp(similar_timestamp, 50);

        assert_eq!(x_value_1, x_value_2);
    }
}

fn build_raw_vecs(metrics_view: &MetricView, resolution: Milliseconds) -> Vec<Vec<(f64, f64)>> {
    let mut data_vecs: Vec<_> = Vec::new();
    let metrics_cardinality = metrics_view.last_or_default().cardinality();

    for dimension_idx in 0..metrics_cardinality {
        let data: Vec<_> = metrics_view
            .as_slice()
            .iter()
            .map(|dm| {
                (
                    calculate_x_value_of_timestamp(dm.timestamp, resolution),
                    dm.metric
                        .as_f64(dimension_idx)
                        .expect("Error accessing raw metric value"),
                )
            })
            .collect();

        data_vecs.push(data);
    }

    data_vecs
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

#[cfg(test)]
mod test_raw_data_from_metrics_view {
    use std::time::Duration;

    use crate::core::collection::ProcessData;
    use crate::core::metrics::IOMetric;
    use crate::core::time::test_utils::advance_time_and_refresh_timestamp;
    use crate::core::time::{Span, Timestamp};
    use crate::ui::chart::build_raw_vecs;

    #[test]
    fn test_should_assign_correct_iteration_to_each_metric() {
        let origin_ts = Timestamp::now();
        let mut process_data = ProcessData::<IOMetric>::new();
        process_data.push(IOMetric::new(10, 20));
        advance_time_and_refresh_timestamp(Duration::from_secs(1));
        process_data.push(IOMetric::new(30, 40));

        let metrics_view = process_data.view(Span::new(origin_ts, Timestamp::now()));
        let raw_vecs = build_raw_vecs(&metrics_view, 1);

        assert_eq!(
            raw_vecs,
            vec![vec![(-1000.0, 10.0), (0.0, 30.0)], vec![(-1000.0, 20.0), (0.0, 40.0)]]
        );
    }
}
