use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::core::metrics::{Archive, ArchiveBuilder, Metric, Probe};
use crate::core::process_view::ProcessView;
use crate::Error;
use crate::procfs::cpu::CpuProbe;
use crate::triggers::Trigger;
use crate::ui::SpvUI;
use crate::ui::Terminal;

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
    terminal: Terminal,
    process_view: ProcessView,
    metrics: Archive,
    ui: SpvUI,
    probe: CpuProbe,
}

// TODO bundle and inject Terminal/SpvUI, to only have one object rendering app from Archive instance
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
            terminal: Terminal::new().map_err(|e| Error::UiError(e.to_string()))?,
            process_view: context.unpack(),
            metrics: archive,
            ui: SpvUI::default(),
            probe: CpuProbe::new().expect("... TODO get rid of this POC"), // TODO
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        self.update_metrics();

        loop {
            let trigger = self.receiver.recv()
                .map_err(|e| Error::MpscError(e.to_string()))?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => self.refresh()?,
                Trigger::NextProcess => {
                    self.ui.next_process();
                    self.draw_ui();
                }
                Trigger::PreviousProcess => {
                    self.ui.previous_process();
                    self.draw_ui();
                }
            }
        }

        Ok(())
    }

    fn refresh(&mut self) -> Result<(), Error> {
        self.update_metrics()?;

        self.draw_ui()?;

        Ok(())
    }

    fn update_metrics(&mut self) -> Result<(), Error> {
        // 1. Get processes
        // 2. Probe metrics for all processes
        // 3. Render
        // How to pass all required info to renderer ?
        //  - it accesses it itself as it has references to MetricsArchive and ProcessSnapshot
        //  - the informations are passed as parameters to render
        let processes = self.process_view
            .sorted_processes(&self.metrics, self.ui.current_tab())
            .map_err(|e| Error::CoreError(e.to_string()))?;

        let pids = processes.iter()
            .map(|pm| pm.pid()).collect();

        let metrics = self.probe.probe_processes(&pids)
            .map_err(|e| Error::CoreError(e.to_string()))?;

        metrics.into_iter()
            .for_each(|(pid, metric)| {
                self.metrics.push(self.ui.current_tab(), pid, metric)
                    .expect("todo get rid of this poc..") // TODO
            });

        self.ui.set_processes(processes);

        Ok(())
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let ui = &mut self.ui;
        let metrics = &self.metrics;
        self.terminal.draw(|f| ui.render(f, metrics))
            .map_err(|e| Error::UiError(e.to_string()));

        Ok(())
    }
}