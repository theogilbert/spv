use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::process::{ProcessCollector, ProcessMetadata, Status};
use crate::core::time::refresh_current_timestamp;
use crate::ctrl::processes::ProcessSelector;
use crate::ctrl::span::RenderingSpan;
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_collector: ProcessCollector,
    ui: SpvUI,
    collectors: HashMap<String, Box<dyn MetricCollector>>,
    rendering_span: RenderingSpan,
    process_selector: ProcessSelector,
}

impl SpvApplication {
    pub fn new(
        receiver: Receiver<Trigger>,
        collectors: Vec<Box<dyn MetricCollector>>,
        process_collector: ProcessCollector,
        impulse_tolerance: Duration,
    ) -> Result<Self, Error> {
        let time_tolerance = 2 * impulse_tolerance;
        const DEFAULT_REPRESENTED_SPAN_DURATION: Duration = Duration::from_secs(60);

        let ui = SpvUI::new(collectors.iter().map(|p| p.name().to_string()), time_tolerance)?;
        let collectors_map = collectors.into_iter().map(|mc| (mc.name().to_string(), mc)).collect();

        Ok(Self {
            receiver,
            process_collector,
            ui,
            collectors: collectors_map,
            rendering_span: RenderingSpan::new(DEFAULT_REPRESENTED_SPAN_DURATION, time_tolerance),
            process_selector: ProcessSelector::default(),
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.calibrate_probes()?;

        loop {
            let trigger = self.receiver.recv()?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => {
                    self.increment_iteration();
                    self.collect_metrics()?;
                }
                Trigger::NextProcess => self.process_selector.next_process(),
                Trigger::PreviousProcess => self.process_selector.previous_process(),
                Trigger::Resize => (), // No need to do anything, just receiving a signal will refresh UI at the end of the loop
                Trigger::NextTab => self.ui.next_tab(),
                Trigger::PreviousTab => self.ui.previous_tab(),
                Trigger::ScrollLeft => self.rendering_span.scroll_left(),
                Trigger::ScrollRight => self.rendering_span.scroll_right(),
                Trigger::ScrollReset => self.rendering_span.reset_scroll(),
            }

            self.draw_ui()?;
        }

        Ok(())
    }

    fn increment_iteration(&mut self) {
        refresh_current_timestamp();
        self.rendering_span.refresh();
    }

    fn calibrate_probes(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let pids = self.process_collector.running_pids();

        for c in self.collectors.values_mut() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let running_pids = self.process_collector.running_pids();

        for collector in self.collectors.values_mut() {
            collector.collect(&running_pids).unwrap_or_else(|e| {
                warn!("Error reading from collector {}: {}", collector.name(), e.to_string());
            });
        }

        let mut exposed_processes = self.represented_processes();
        self.sort_processes_by_status_and_metric(&mut exposed_processes);
        self.process_selector.set_processes(exposed_processes);

        Ok(())
    }

    fn scan_processes(&mut self) -> Result<(), Error> {
        self.process_collector.collect_processes().map_err(Error::CoreError)
    }

    fn represented_processes(&self) -> Vec<ProcessMetadata> {
        // TODO selected process should be represented even if it expired
        let rendered_span = self.rendering_span.to_span();

        self.process_collector
            .processes()
            .into_iter()
            .filter(|pm| pm.running_span().intersects(&rendered_span))
            .collect()
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

    fn draw_ui(&mut self) -> Result<(), Error> {
        let metrics_collector = self.current_collector(&self.collectors);

        let processes = self.process_selector.to_view();
        let overview = metrics_collector.overview();

        let metrics_view = self
            .process_selector
            .selected_process()
            .map(|pm| metrics_collector.view(pm.pid(), self.rendering_span.to_span()));

        self.ui
            .render(&overview, &metrics_view, &processes)
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
