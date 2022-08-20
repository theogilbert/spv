//! Interprets user inputs
use std::time::Duration;

use crate::core::collection::MetricCollector;
use crate::core::ordering::ProcessOrdering;
use crate::core::process::ProcessMetadata;
use crate::core::time::Span;
use crate::core::view::{CollectorsView, ProcessesView};
use crate::ctrl::collectors::Collectors;
use crate::ctrl::processes::{ProcessSelector, SortCriteriaSelector};
use crate::ctrl::span::RenderingSpan;
use crate::triggers::Key;

pub mod collectors;
pub mod processes;
pub mod span;

/// Indicates the effect caused by a user input
#[derive(Eq, PartialEq)]
pub enum Effect {
    None,
    ProcessesSorted(ProcessOrdering),
}

#[derive(Copy, Clone)]
pub enum State {
    Spv,
    SortingPrompt(ProcessOrdering),
}

/// Wraps all controls utilities within a single unit
pub struct Controls {
    collectors: Collectors,
    rendering_span: RenderingSpan,
    process_selector: ProcessSelector,
    sort_criteria_selector: SortCriteriaSelector,
    current_state: State,
}

impl Controls {
    pub fn new(
        collectors: Vec<Box<dyn MetricCollector>>,
        initial_span_duration: Duration,
        span_tolerance: Duration,
    ) -> Self {
        Self {
            collectors: Collectors::new(collectors),
            rendering_span: RenderingSpan::new(initial_span_duration, span_tolerance),
            process_selector: ProcessSelector::default(),
            sort_criteria_selector: SortCriteriaSelector::default(),
            current_state: State::Spv,
        }
    }

    /// Interprets the user input to control the application.
    /// The input will have a different effect depending on the state of the application.
    ///
    /// Returns the effect caused by the input.
    pub fn interpret_input(&mut self, input: Key) -> Effect {
        match self.current_state {
            State::Spv => self.interpret_spv_input(input),
            State::SortingPrompt(_) => self.interpret_sorting_prompt_input(input),
        }
    }

    fn interpret_spv_input(&mut self, input: Key) -> Effect {
        match input {
            Key::P => self.collectors.previous_collector(),
            Key::N => self.collectors.next_collector(),
            Key::Up => self.process_selector.previous_process(),
            Key::Down => self.process_selector.next_process(),
            Key::G => self.rendering_span.reset_scroll(),
            Key::Left => self.rendering_span.scroll_left(),
            Key::Right => self.rendering_span.scroll_right(),
            Key::S => self.current_state = State::SortingPrompt(self.sort_criteria_selector.applied()),
            _ => {}
        }

        Effect::None
    }

    fn interpret_sorting_prompt_input(&mut self, input: Key) -> Effect {
        match input {
            Key::S | Key::Escape => self.current_state = State::Spv,
            Key::Down => {
                self.sort_criteria_selector.next();
                self.refresh_state();
            }
            Key::Up => {
                self.sort_criteria_selector.previous();
                self.refresh_state();
            }
            Key::Submit => {
                self.sort_criteria_selector.apply();
                self.current_state = State::Spv;
                return Effect::ProcessesSorted(self.sort_criteria_selector.applied());
            }
            _ => {} // In this state, most user inputs are ignored
        }

        Effect::None
    }

    fn refresh_state(&mut self) {
        if let State::SortingPrompt(_) = self.current_state {
            self.current_state = State::SortingPrompt(self.sort_criteria_selector.selected());
        }
    }

    pub fn refresh_span(&mut self) {
        self.rendering_span.refresh();
    }

    pub fn to_span(&self) -> Span {
        self.rendering_span.to_span()
    }

    pub fn set_processes(&mut self, processes: Vec<ProcessMetadata>) {
        self.process_selector.set_processes(processes);
    }

    pub fn to_processes_view(&self) -> ProcessesView {
        self.process_selector.to_view()
    }

    pub fn collectors_as_mut_slice(&mut self) -> &mut [Box<dyn MetricCollector>] {
        self.collectors.as_mut_slice()
    }

    pub fn current_collector(&self) -> &dyn MetricCollector {
        self.collectors.current()
    }

    pub fn to_collectors_view(&self) -> CollectorsView {
        self.collectors.to_view()
    }

    pub fn state(&self) -> State {
        self.current_state
    }

    pub fn process_ordering_criteria(&self) -> ProcessOrdering {
        self.sort_criteria_selector.applied()
    }
}
