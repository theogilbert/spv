//! Manages the selection of which type of metrics to render

use crate::core::collection::MetricCollector;
use crate::core::view::CollectorsView;

/// Contains the list of collectors available to the application,
/// and allow a user from selecting one
pub struct Collectors {
    collectors: Vec<Box<dyn MetricCollector>>,
    selected_index: usize,
}

impl Collectors {
    /// Builds a new metric collectors selector
    ///
    /// Panics if no collectors are given in parameter
    pub fn new(collectors: Vec<Box<dyn MetricCollector>>) -> Self {
        if collectors.is_empty() {
            panic!("No collectors have been defined");
        }
        Self {
            collectors,
            selected_index: 0,
        }
    }

    pub fn next_collector(&mut self) {
        self.selected_index = (self.selected_index + 1) % self.collectors.len();
    }

    pub fn previous_collector(&mut self) {
        self.selected_index = self.selected_index.checked_sub(1).unwrap_or(self.collectors.len() - 1);
    }

    pub fn current(&self) -> &dyn MetricCollector {
        self.collectors[self.selected_index].as_ref()
    }

    pub fn as_mut_slice(&mut self) -> &mut [Box<dyn MetricCollector>] {
        self.collectors.as_mut_slice()
    }

    pub fn to_view(&self) -> CollectorsView {
        let names = self.collectors.iter().map(|mc| mc.name()).collect();
        CollectorsView::new(names, self.selected_index)
    }
}

#[cfg(test)]
mod test_collectors_selector {
    use std::cmp::Ordering;

    use rstest::*;

    use crate::core::collection::MetricCollector;
    use crate::core::process::Pid;
    use crate::core::time::Span;
    use crate::core::view::{MetricView, MetricsOverview};
    use crate::core::Error;
    use crate::ctrl::collectors::Collectors;

    struct FakeCollector {
        name: &'static str,
    }

    impl MetricCollector for FakeCollector {
        fn collect(&mut self, _pids: &[Pid]) -> Result<(), Error> {
            unimplemented!()
        }

        fn cleanup(&mut self, _pids: &[Pid]) {
            unimplemented!()
        }

        fn calibrate(&mut self, _pids: &[Pid]) -> Result<(), Error> {
            unimplemented!()
        }

        fn compare_pids_by_last_metrics(&self, _pid1: Pid, _pid2: Pid) -> Ordering {
            unimplemented!()
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn view(&self, _pid: Pid, _span: Span) -> MetricView {
            unimplemented!()
        }

        fn overview(&self) -> MetricsOverview {
            unimplemented!()
        }
    }

    #[fixture]
    fn collectors() -> Vec<Box<dyn MetricCollector>> {
        vec![
            Box::new(FakeCollector { name: "collector_1" }),
            Box::new(FakeCollector { name: "collector_2" }),
        ]
    }

    #[rstest]
    #[should_panic]
    fn test_should_panic_when_no_selectors_given() {
        Collectors::new(vec![]);
    }

    #[rstest]
    fn test_should_select_first_collector_by_default(collectors: Vec<Box<dyn MetricCollector>>) {
        let selector = Collectors::new(collectors);

        assert_eq!(selector.current().name(), "collector_1");
    }

    #[rstest]
    fn test_should_select_next_collector(collectors: Vec<Box<dyn MetricCollector>>) {
        let mut selector = Collectors::new(collectors);
        selector.next_collector();

        assert_eq!(selector.current().name(), "collector_2");
    }

    #[rstest]
    fn test_should_select_prev_collector(collectors: Vec<Box<dyn MetricCollector>>) {
        let mut selector = Collectors::new(collectors);
        selector.next_collector();
        selector.previous_collector();

        assert_eq!(selector.current().name(), "collector_1");
    }

    #[rstest]
    fn test_should_select_first_collector_after_last(collectors: Vec<Box<dyn MetricCollector>>) {
        let mut selector = Collectors::new(collectors);
        selector.next_collector();
        selector.next_collector();

        assert_eq!(selector.current().name(), "collector_1");
    }

    #[rstest]
    fn test_should_select_last_collector_before_first(collectors: Vec<Box<dyn MetricCollector>>) {
        let mut selector = Collectors::new(collectors);
        selector.previous_collector();

        assert_eq!(selector.current().name(), "collector_2");
    }

    #[rstest]
    fn test_should_build_correct_view(collectors: Vec<Box<dyn MetricCollector>>) {
        let mut selector = Collectors::new(collectors);
        selector.next_collector();
        let view = selector.to_view();

        assert_eq!(view.selected_index(), 1);
        assert_eq!(view.collectors_names(), ["collector_1", "collector_2"])
    }
}
