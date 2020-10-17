use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::error;

use crate::core::metrics::{Archive, ArchiveBuilder, Probe};
use crate::core::process_view::{ProcessView, PID};
use crate::Error;
use crate::triggers::Trigger;
use crate::ui::SpvUI;

pub struct SpvContext {
    process_view: ProcessView
}

impl SpvContext {
    pub fn new(process_view: ProcessView) -> Self {
        Self { process_view }
    }

    pub fn unpack(self) -> ProcessView {
        self.process_view
    }
}


pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    process_view: ProcessView,
    metrics_archive: Archive,
    ui: SpvUI,
    probes: Vec<Box<dyn Probe>>,
}


impl SpvApplication {
    pub fn new(receiver: Receiver<Trigger>, probes: Vec<Box<dyn Probe>>, context: SpvContext, probe_period: Duration) -> Result<Self, Error> {
        let mut builder = ArchiveBuilder::new();

        for p in probes.iter() {
            builder = builder.new_metric(p.name().to_string(), p.default_metric())
                .map_err(|ce| Error::CoreError(ce.to_string()))?;
        }

        let archive = builder
            .resolution(probe_period)
            .build();

        let ui = SpvUI::new(probes.iter()
            .map(|p| p.name().to_string()))
            .map_err(|e| Error::UiError(e.to_string()))?;

        Ok(Self {
            receiver,
            process_view: context.unpack(),
            metrics_archive: archive,
            ui,
            probes,
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.dispatch_probes()?;

        loop {
            let trigger = self.receiver.recv()
                .map_err(|e| Error::MpscError(e.to_string()))?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => {
                    self.dispatch_probes()?
                }
                Trigger::NextProcess => self.ui.next_process(),
                Trigger::PreviousProcess => self.ui.previous_process(),
                Trigger::Resize => {}
            }

            self.draw_ui()?;
        }

        Ok(())
    }

    fn dispatch_probes(&mut self) -> Result<(), Error> {
        let mut processes = self.process_view.processes()
            .map_err(|e| Error::CoreError(e.to_string()))?;

        let pids = processes.iter()
            .map(|p| p.pid())
            .collect();

        let probes = &mut self.probes;
        let archive = &mut self.metrics_archive;
        let current_tab = self.ui.current_tab();
        for p in probes {
            Self::probe_metrics(p, &pids, archive, current_tab)?;
        }

        // TODO processes should be its own type of object, with a sort() method instead..
        ProcessView::sort_processes(&mut processes, &self.metrics_archive,
                                    self.ui.current_tab());
        self.ui.set_processes(processes);

        Ok(())
    }

    fn probe_metrics(probe: &mut Box<dyn Probe>, pids: &Vec<PID>, archive: &mut Archive, current_tab: &str) -> Result<(), Error> {
        let metrics = probe.probe_processes(pids)
            .map_err(|e| Error::CoreError(e.to_string()))?;

        let metric_label = current_tab;
        for (pid, m) in metrics.into_iter() {
            archive.push(metric_label, pid, m)
                .map_err(|e| {
                    error!("Error pushing {} metric for PID {}: {}", metric_label, pid, e);
                    e
                })
                .expect(&format!("Error pushing {} metric for PID {}", metric_label, pid));
        }

        Ok(())
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        self.ui.render(&self.metrics_archive)
            .map_err(|e| Error::UiError(e.to_string()))
    }
}