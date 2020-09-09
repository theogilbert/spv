use std::io;
use std::io::Stdout;
use std::sync::mpsc::Receiver;

use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::Terminal;

use crate::triggers::Trigger;
use crate::ui::FrameRenderer;

pub type TuiBackend = TermionBackend<RawTerminal<Stdout>>;

pub enum Error {
    MpscError(String),
    IOError(String),
}

pub struct SpvContext;


impl SpvContext {
    pub fn new() -> Self {
        Self {}
    }
}


pub struct SpvApplication {
    receiver: Receiver<Trigger>,
    context: SpvContext,
}


impl SpvApplication {
    pub fn new(receiver: Receiver<Trigger>, context: SpvContext) -> Self {
        Self { receiver, context }
    }

    pub fn run(self) -> Result<(), Error> {
        Self::nice_screen_clear();

        let mut terminal = SpvApplication::init_terminal()?;
        let mut renderer = FrameRenderer::new();

        loop {
            let trigger = self.receiver.recv()
                .map_err(|e| Error::MpscError(e.to_string()))?;

            match trigger {
                Trigger::Exit => break,
                Trigger::Impulse => {
                    // 1. Get processes
                    // 2. Probe metrics for all processes
                    // 3. Render
                    // How to pass all required info to renderer ?
                    //  - it accesses it itself as it has references to MetricsArchive and ProcessSnapshot
                    //  - the informations are passed as parameters to render
                    terminal.draw(|f| renderer.render(f));
                }
            }
        }

        terminal.clear();

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