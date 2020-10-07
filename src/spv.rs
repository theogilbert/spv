use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::critical;

use crate::core::metrics::{Archive, ArchiveBuilder, Metric, Probe};
use crate::core::process_view::ProcessView;
use crate::Error;
use crate::procfs::cpu::CpuProbe;
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
    metrics: Archive,
    ui: SpvUI,
    probe: CpuProbe,
}


impl SpvApplication {
    pub fn new(receiver: Receiver<Trigger>, context: SpvContext, probe_period: Duration) -> Result<Self, Error> {
        let archive = ArchiveBuilder::new()
            .resolution(probe_period)
            .new_metric("CPU Usage".to_string(),
                        Metric::from_percent(0.).unwrap())
            .unwrap()
            .build();

        Ok(Self {
            receiver,
            process_view: context.unpack(),
            metrics: archive,
            ui: SpvUI::new().map_err(|e| Error::UiError(e.to_string()))?,
            probe: CpuProbe::new().expect("... TODO get rid of this POC"), // TODO
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.probe_metrics();

        loop {
            let trigger = self.receiver.recv()
                .map_err(|e| Error::MpscError(e.to_string()))?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => {
                    self.probe_metrics()?
                }
                Trigger::NextProcess => self.ui.next_process(),
                Trigger::PreviousProcess => self.ui.previous_process(),
                Trigger::Resize => {}
            }

            self.draw_ui();
        }

        Ok(())
    }

    fn probe_metrics(&mut self) -> Result<(), Error> {
        let processes = self.process_view.processes()
            .map_err(|e| Error::CoreError(e.to_string()))?;

        let pids = processes.iter()
            .map(|pm| pm.pid()).collect();

        let metrics = self.probe.probe_processes(&pids)
            .map_err(|e| Error::CoreError(e.to_string()))?;

        metrics.into_iter()
            .for_each(|(pid, metric)| {
                let metric_label = self.ui.current_tab();
                self.metrics.push(metric_label, pid, metric)
                    .map_err(|e| {
                        critical!("Error pushing {} metric for PID {}: {}", metric_label, pid, e);
                        e
                    })
                    .expect(&format!("Error pushing {} metric for PID {}", metric_label, pid));
            });

        // TODO processes should be its own type of object, with a sort() method instead..
        let processes = ProcessView::sort_processes(processes, &self.metrics, self.ui.current_tab())
            .map_err(|e| Error::CoreError(e.to_string()))?;
        self.ui.set_processes(processes);

        Ok(())
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        self.ui.render(&self.metrics)
            .map_err(|e| Error::UiError(e.to_string()))
    }
}