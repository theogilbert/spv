//! Integrates all other modules to run spv

use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::warn;

use crate::core::collection::MetricCollector;
use crate::core::ordering::sort_processes;
use crate::core::process::{ProcessCollector, ProcessMetadata};
use crate::core::time::refresh_current_timestamp;
use crate::ctrl::{Controls, Effect};
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::Error;

pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_collector: ProcessCollector,
    ui: SpvUI,
    controls: Controls,
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
            controls: Controls::new(collectors, DEFAULT_REPRESENTED_SPAN_DURATION, 2 * impulse_tolerance),
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
                Trigger::Resize => (), // No need to do anything, just receiving a signal will refresh UI at the end of the loop
                Trigger::Input(input) => {
                    let effect = self.controls.interpret_input(input);
                    if effect != Effect::None {
                        self.ui.set_status_from_effect(effect);
                    }
                }
            }

            self.draw_ui()?;
        }

        Ok(())
    }

    fn increment_iteration(&mut self) {
        refresh_current_timestamp();
        self.controls.refresh_span();
    }

    fn calibrate_probes(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let pids = self.process_collector.running_pids();

        for c in self.controls.collectors_as_mut_slice() {
            c.calibrate(&pids)?;
        }

        Ok(())
    }

    fn collect_metrics(&mut self) -> Result<(), Error> {
        self.scan_processes()?;
        let running_pids = self.process_collector.running_pids();

        for collector in self.controls.collectors_as_mut_slice() {
            collector.collect(&running_pids).unwrap_or_else(|e| {
                warn!("Error reading from collector {}: {}", collector.name(), e.to_string());
            });
        }

        let mut exposed_processes = self.represented_processes();
        sort_processes(
            &mut exposed_processes,
            self.controls.process_ordering_criteria(),
            self.controls.current_collector(),
        );
        self.controls.set_processes(exposed_processes);

        Ok(())
    }

    fn scan_processes(&mut self) -> Result<(), Error> {
        let collection_ret = self.process_collector.collect_processes().map_err(Error::CoreError);

        let dead_processes = self.process_collector.latest_dead_processes();
        for collector in self.controls.collectors_as_mut_slice() {
            collector.cleanup(&dead_processes);
        }

        collection_ret
    }

    fn represented_processes(&self) -> Vec<ProcessMetadata> {
        // TODO selected process should be represented even if it expired
        let rendered_span = self.controls.to_span();

        self.process_collector
            .processes()
            .into_iter()
            .filter(|pm| pm.running_span().intersects(&rendered_span))
            .collect()
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let collectors = self.controls.to_collectors_view();
        let processes = self.controls.to_processes_view();

        // TODO move overview building code to Controls module
        let current_collector = self.controls.current_collector();
        let overview = current_collector.overview();
        let metrics_view = processes
            .selected_process()
            .map(|pm| current_collector.view(pm.pid(), self.controls.to_span()));

        // TODO wrap all these views/state in a standalone structure (or pass Controls) ?
        self.ui
            .render(
                &collectors,
                &processes,
                &overview,
                metrics_view.as_ref(),
                self.controls.state(),
            )
            .map_err(Error::UiError)
    }
}
