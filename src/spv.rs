use std::cmp::Ordering;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::process::{ProcessCollector, ProcessMetadata, Status};
use crate::core::time::refresh_current_timestamp;
use crate::ctrl::collectors::Collectors;
use crate::ctrl::processes::ProcessSelector;
use crate::ctrl::span::RenderingSpan;
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_collector: ProcessCollector,
    ui: SpvUI,
    collectors: Collectors,
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
        const DEFAULT_REPRESENTED_SPAN_DURATION: Duration = Duration::from_secs(60);

        Ok(Self {
            receiver,
            process_collector,
            ui: SpvUI::new(2 * impulse_tolerance)?,
            collectors: Collectors::new(collectors),
            rendering_span: RenderingSpan::new(DEFAULT_REPRESENTED_SPAN_DURATION, 2 * impulse_tolerance),
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
                Trigger::NextTab => self.collectors.next_collector(),
                Trigger::PreviousTab => self.collectors.previous_collector(),
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

        for c in self.collectors.as_mut_slice() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let running_pids = self.process_collector.running_pids();

        for collector in self.collectors.as_mut_slice() {
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
                .collectors
                .current()
                .compare_pids_by_last_metrics(pm1.pid(), pm2.pid())
                .reverse(),
        });
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let collectors = self.collectors.to_view();
        let processes = self.process_selector.to_view();

        let current_collector = self.collectors.current_mut();
        let overview = current_collector.overview();
        let metrics_view = self
            .process_selector
            .selected_process()
            .map(|pm| current_collector.view(pm.pid(), self.rendering_span.to_span()));

        self.ui
            .render(&collectors, &processes, &overview, metrics_view.as_ref())
            .map_err(Error::UiError)
    }
}
