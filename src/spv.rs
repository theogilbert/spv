use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::process::{ProcessCollector, ProcessMetadata, Status};
use crate::core::time::{refresh_current_timestamp, Span, Timestamp};
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_view: ProcessCollector,
    ui: SpvUI,
    collectors: HashMap<String, Box<dyn MetricCollector>>,
    represented_span: Span,
}

impl SpvApplication {
    pub fn new(
        receiver: Receiver<Trigger>,
        collectors: Vec<Box<dyn MetricCollector>>,
        process_view: ProcessCollector,
        impulse_tolerance: Duration,
    ) -> Result<Self, Error> {
        let time_tolerance = 2 * impulse_tolerance;
        let ui = SpvUI::new(collectors.iter().map(|p| p.name().to_string()), time_tolerance)?;

        let collectors_map = collectors.into_iter().map(|mc| (mc.name().to_string(), mc)).collect();

        const DEFAULT_REPRESENTED_SPAN_DURATION: Duration = Duration::from_secs(60);
        let mut rendered_span = Span::from_duration(DEFAULT_REPRESENTED_SPAN_DURATION);
        rendered_span.set_tolerance(time_tolerance);

        Ok(Self {
            receiver,
            process_view,
            ui,
            collectors: collectors_map,
            represented_span: rendered_span,
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.calibrate_probes()?;
        self.collect_metrics()?;

        loop {
            let trigger = self.receiver.recv()?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => {
                    self.increment_iteration();
                    self.collect_metrics()?;
                }
                Trigger::NextProcess => self.ui.next_process(),
                Trigger::PreviousProcess => self.ui.previous_process(),
                Trigger::Resize => (), // No need to do anything, just receiving a signal will refresh UI at the end of the loop
                Trigger::NextTab => self.ui.next_tab(),
                Trigger::PreviousTab => self.ui.previous_tab(),
                Trigger::ScrollLeft => self.represented_span.scroll_left(Duration::from_secs(1)),
                Trigger::ScrollRight => self.represented_span.scroll_right(Duration::from_secs(1)),
                Trigger::ScrollReset => self.represented_span.set_end_and_shift(Timestamp::now()),
            }

            self.draw_ui()?;
        }

        Ok(())
    }

    fn increment_iteration(&mut self) {
        let span_should_follow_current_iteration = self.represented_span.is_fully_scrolled_right();

        refresh_current_timestamp();

        if span_should_follow_current_iteration {
            self.represented_span.set_end_and_shift(Timestamp::now());
        }
    }

    fn calibrate_probes(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let pids = self.process_view.running_pids();

        for c in self.collectors.values_mut() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let running_pids = self.process_view.running_pids();

        for collector in self.collectors.values_mut() {
            collector.collect(&running_pids).unwrap_or_else(|e| {
                warn!("Error reading from collector {}: {}", collector.name(), e.to_string());
            });
        }

        let mut exposed_processes = self.represented_processes();

        self.sort_processes_by_status_and_metric(&mut exposed_processes);
        self.ui.set_processes(exposed_processes);

        Ok(())
    }

    fn scan_processes(&mut self) -> Result<(), Error> {
        self.process_view.collect_processes().map_err(Error::CoreError)?;
        Ok(())
    }

    fn represented_processes(&self) -> Vec<ProcessMetadata> {
        // TODO selected process should be represented even if it expired
        self.process_view
            .processes()
            .into_iter()
            .filter(|pm| pm.running_span().intersects(&self.represented_span))
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
        let current_collector = self.current_collector(&self.collectors);

        let overview = current_collector.overview();

        let metrics_view = self
            .ui
            .current_process()
            .map(|pm| current_collector.view(pm, self.represented_span));

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
