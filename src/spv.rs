use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::core::metrics::{Archive, ArchiveBuilder, Probe};
use crate::core::process_view::{PID, ProcessMetadata, ProcessView};
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

        let mut spv_app = Self {
            receiver,
            process_view: context.unpack(),
            metrics_archive: archive,
            ui,
            probes,
        };

        spv_app.calibrate_probes()?;
        spv_app.dispatch_probes()?;

        Ok(spv_app)
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

    fn calibrate_probes(&mut self) -> Result<(), Error> {
        let processes = self.collect_processes()?;
        let pids = SpvApplication::extract_processes_pids(&processes);

        for p in &mut self.probes {
            p.probe_processes(&pids)
                .map_err(|e| Error::CoreError(e.to_string()))?;
        }

        Ok(())
    }

    fn dispatch_probes(&mut self) -> Result<(), Error> {
        let mut processes = self.collect_processes()?;
        let pids = SpvApplication::extract_processes_pids(&processes);

        let probes = &mut self.probes;
        let archive = &mut self.metrics_archive;
        let current_tab = self.ui.current_tab();
        for p in probes {
            Self::probe_metrics(p, &pids, archive, current_tab)?;
        }

        // TODO processes should be its own type of object, with a sort() method instead..
        //  Or should it ?
        ProcessView::sort_processes(&mut processes, &self.metrics_archive,
                                    self.ui.current_tab());
        self.ui.set_processes(processes);

        Ok(())
    }

    fn extract_processes_pids(processes: &Vec<ProcessMetadata>) -> Vec<u32> {
        processes.iter()
            .map(|p| p.pid())
            .collect()
    }

    fn collect_processes(&mut self) -> Result<Vec<ProcessMetadata>, Error> {
        self.process_view.processes()
            .map_err(|e| Error::CoreError(e.to_string()))
    }

    fn probe_metrics(probe: &mut Box<dyn Probe>, pids: &[PID], archive: &mut Archive, current_tab: &str) -> Result<(), Error> {
        let metrics = probe.probe_processes(pids)
            .map_err(|e| Error::CoreError(e.to_string()))?;

        let metric_label = current_tab;
        for (pid, m) in metrics.into_iter() {
            archive.push(metric_label, pid, m)
                .unwrap_or_else(|_| panic!("Error pushing {} metric for PID {}", metric_label, pid))
        }

        Ok(())
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        self.ui.render(&self.metrics_archive)
            .map_err(|e| Error::UiError(e.to_string()))
    }
}