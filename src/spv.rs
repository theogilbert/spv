use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::process::{ProcessCollector, ProcessMetadata, Status};
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_view: ProcessCollector,
    ui: SpvUI,
    collectors: HashMap<String, Box<dyn MetricCollector>>,
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
        let processes = self.collect_processes()?;
        let pids = SpvApplication::extract_processes_pids(&processes);

        for c in self.collectors.values_mut() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        let mut processes = self.collect_processes()?;
        let pids = SpvApplication::extract_processes_pids(&processes);

        for collector in self.collectors.values_mut() {
            collector.collect(&pids).unwrap_or_else(|e| {
                warn!("Error reading from collector {}: {}", collector.name(), e);
            });
        }

        self.sort_processes_by_status_and_metric(&mut processes);
        self.ui.set_processes(processes);

        Ok(())
    }

    fn sort_processes_by_status_and_metric(&self, processes: &mut Vec<ProcessMetadata>) {
        processes.sort_by(|pm1, pm2| match (pm1.status(), pm2.status()) {
            (Status::RUNNING, Status::DEAD) => Ordering::Less,
            (Status::DEAD, Status::RUNNING) => Ordering::Greater,
            (_, _) => self
                .current_collector(&self.collectors)
                .compare_pids_by_last_metrics(pm1.pid(), pm2.pid())
                .reverse(),
        });
    }

    fn collect_processes(&mut self) -> Result<Vec<ProcessMetadata>, Error> {
        self.process_view.collect_processes().map_err(Error::CoreError)
    }

    fn extract_processes_pids(processes: &[ProcessMetadata]) -> Vec<u32> {
        processes.iter().map(|pm| pm.pid()).collect()
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let selected_pid = self.ui.current_process().map_or(0, |pm| pm.pid());

        let current_collector = self.current_collector(&self.collectors);

        self.ui
            .render(&current_collector.overview(), &current_collector.view(selected_pid))
            .map_err(Error::UiError)
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
