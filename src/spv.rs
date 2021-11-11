use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::iteration::{IterSpan, IterationTracker};
use crate::core::process::{ProcessCollector, ProcessMetadata, Status};
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_view: ProcessCollector,
    ui: SpvUI,
    collectors: HashMap<String, Box<dyn MetricCollector>>,
    iteration_tracker: IterationTracker,
    iteration_span: IterSpan,
}

impl SpvApplication {
    pub fn new(
        receiver: Receiver<Trigger>,
        collectors: Vec<Box<dyn MetricCollector>>,
        process_view: ProcessCollector,
    ) -> Result<Self, Error> {
        let ui = SpvUI::new(collectors.iter().map(|p| p.name().to_string()))?;

        let collectors_map = collectors.into_iter().map(|mc| (mc.name().to_string(), mc)).collect();

        let mut spv_app = Self {
            receiver,
            process_view,
            ui,
            collectors: collectors_map,
            iteration_tracker: IterationTracker::default(),
            iteration_span: IterSpan::default(),
        };

        spv_app.calibrate_probes()?;
        spv_app.collect_metrics()?;

        Ok(spv_app)
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.collect_metrics()?;

        loop {
            let trigger = self.receiver.recv()?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => self.collect_metrics()?,
                Trigger::NextProcess => self.ui.next_process(),
                Trigger::PreviousProcess => self.ui.previous_process(),
                Trigger::Resize => {
                    // No need to do anything, just receiving a signal will refresh UI
                }
                Trigger::NextTab => self.ui.next_tab(),
                Trigger::PreviousTab => self.ui.previous_tab(),
            }

            self.draw_ui()?;
        }

        Ok(())
    }

    fn calibrate_probes(&mut self) -> Result<(), Error> {
        self.collect_running_processes()?;
        let pids = Self::extract_processes_pids(&self.process_view.running_processes());

        for c in self.collectors.values_mut() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        self.iteration_tracker.tick();

        self.collect_running_processes()?;
        let running_pids = Self::extract_processes_pids(&self.process_view.running_processes());

        for collector in self.collectors.values_mut() {
            collector
                .collect(&running_pids, self.iteration_tracker.current())
                .unwrap_or_else(|e| {
                    warn!("Error reading from collector {}: {}", collector.name(), e.to_string());
                });
        }

        let mut exposed_processes = self.exposed_processes();

        self.sort_processes_by_status_and_metric(&mut exposed_processes);
        self.ui.set_processes(exposed_processes);

        Ok(())
    }

    fn collect_running_processes(&mut self) -> Result<(), Error> {
        self.process_view
            .collect_processes(self.iteration_tracker.current())
            .map_err(Error::CoreError)?;
        Ok(())
    }

    fn extract_processes_pids(processes: &[ProcessMetadata]) -> Vec<u32> {
        processes.iter().map(|pm| pm.pid()).collect()
    }

    fn exposed_processes(&self) -> Vec<ProcessMetadata> {
        // TODO once IterSpan is reworked, use span::intersects(span) to check which processes should be exposed
        let cur_iteration = self.iteration_tracker.current();
        let minimum_represented_iteration = self.iteration_span.begin(self.iteration_tracker.current());

        self.process_view
            .processes()
            .into_iter()
            .filter(|pm| pm.iteration_of_death().unwrap_or(cur_iteration) >= minimum_represented_iteration)
            .collect()
    }

    fn sort_processes_by_status_and_metric(&self, processes: &mut Vec<ProcessMetadata>) {
        // TODO move this to ProcessCollector
        processes.sort_by(|pm1, pm2| match (pm1.status(), pm2.status()) {
            (Status::RUNNING, Status::DEAD) => Ordering::Less,
            (Status::DEAD, Status::RUNNING) => Ordering::Greater,
            (_, _) => self
                .current_collector(&self.collectors)
                .compare_pids_by_last_metrics(pm1.pid(), pm2.pid())
                .reverse(),
        });
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let current_collector = self.current_collector(&self.collectors);

        let overview = current_collector.overview();

        let selected_pid = self.ui.current_process().map(|pm| pm.pid()).unwrap_or(0);
        let current_iteration = self.iteration_tracker.current(); // TODO once IterSpan is reworked, no need to pass current_iteration here anymore
        let metrics_view = current_collector.view(selected_pid, self.iteration_span, current_iteration);

        self.ui.render(&overview, &metrics_view).map_err(Error::UiError)
    }

    fn current_collector<'a>(
        &self,
        collectors: &'a HashMap<String, Box<dyn MetricCollector>>,
    ) -> &'a dyn MetricCollector {
        // The collectors attribute has to be passed as parameters. Otherwise the compiler thinks that
        // this function borrows the whole &self reference immutably (preventing further mutable borrowing of self.ui)
        collectors
            .get(self.ui.current_tab())
            .expect("No collector is selected")
            .as_ref()
    }
}
