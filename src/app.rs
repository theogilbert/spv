use std::io;
use std::io::Stdout;
use std::sync::mpsc::Receiver;

use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::Terminal;

use crate::core::metrics::{Archive, ArchiveBuilder, Metric, Probe};
use crate::core::process_view::ProcessView;
use crate::procfs::cpu::CpuProbe;
use crate::triggers::Trigger;
use crate::ui::SpvUI;

pub type TuiBackend = TermionBackend<RawTerminal<Stdout>>;

#[derive(Debug)]
pub enum Error {
    MpscError(String),
    IOError(String),
    ProcessScanError(String),
    ProcessProbeError(String),
}

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
    terminal: Terminal<TuiBackend>,
    process_view: ProcessView,
    metrics: Archive,
    ui: SpvUI,
    probe: CpuProbe,
}


impl SpvApplication {
    pub fn new(receiver: Receiver<Trigger>, context: SpvContext) -> Result<Self, Error> {
        let archive = ArchiveBuilder::new()
            .new_metric("CPU Usage".to_string(),
                        Metric::from_percent(0.).unwrap())
            .unwrap()
            .build();

        Ok(Self {
            receiver,
            terminal: SpvApplication::init_terminal()?,
            process_view: context.unpack(),
            metrics: archive,
            ui: SpvUI::default(),
            probe: CpuProbe::new().expect("... TODO get rid of this POC"),
        })
    }

    pub fn run(mut self) -> Result<(), Error> {
        Self::nice_screen_clear().ok();
        self.update_metrics();

        loop {
            let trigger = self.receiver.recv()
                .map_err(|e| Error::MpscError(e.to_string()))?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => self.on_impulse()?,
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

        self.terminal.clear().ok();

        Ok(())
    }

    fn draw_ui(&mut self) -> Result<(), Error> {
        let ui = &mut self.ui;
        let metrics = &self.metrics;
        self.terminal.draw(|f| ui.render(f, metrics))
            .map_err(|e| Error::IOError(e.to_string()))?;

        Ok(())
    }

    fn on_impulse(&mut self) -> Result<(), Error> {
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
            .map_err(|e| Error::ProcessScanError(e.to_string()))?;

        let pids = processes.iter()
            .map(|pm| pm.pid()).collect();

        let metrics = self.probe.probe_processes(&pids)
            .map_err(|e| Error::ProcessProbeError(e.to_string()))?;

        metrics.into_iter()
            .for_each(|(pid, metric)| {
                self.metrics.push(self.ui.current_tab(), pid, metric)
                    .expect("todo get rid of this poc..")
            });

        self.ui.set_processes(processes);

        Ok(())
    }

    /// Instead of terminal.clear(), this function will not erase current text in the screen. It
    /// will rather append new line until the next buffer does not cover existing text
    fn nice_screen_clear() -> Result<(), Error> {
        let mut tmp_terminal = Self::init_terminal()?;

        print!("{}", "\n".repeat(tmp_terminal.get_frame().size().height as usize));

        Ok(())
    }

    fn init_terminal() -> Result<Terminal<TuiBackend>, Error> {
        let stdout = io::stdout()
            .into_raw_mode()
            .map_err(|e| Error::IOError(e.to_string()))?;
        let backend = TermionBackend::new(stdout);

        Terminal::new(backend)
            .map_err(|e| Error::IOError(e.to_string()))
    }
}