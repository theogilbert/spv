use std::io;
use std::io::Stdout;

use log::error;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::{Frame, Terminal as TuiTerminal};
use tui::backend::TermionBackend;

use crate::ui::Error;

pub type TuiBackend = TermionBackend<RawTerminal<Stdout>>;


pub struct Terminal {
    tui_terminal: TuiTerminal<TuiBackend>
}

impl Terminal {
    pub fn new() -> Result<Self, Error> {
        let stdout = io::stdout()
            .into_raw_mode()
            .map_err(|e| Error::IOError(e))?;
        let backend = TermionBackend::new(stdout);

        let mut tui_terminal = TuiTerminal::new(backend)
            .map_err(|e| Error::IOError(e))?;

        Self::generate_empty_frame(&mut tui_terminal);

        Ok(Terminal { tui_terminal })
    }

    /// On application startup, calling self.clear() would overwrite the current content of the
    /// terminal. By generating an empty frame instead and then overwriting that frame, we do not
    /// erase any existing content on application startup.
    fn generate_empty_frame(terminal: &mut TuiTerminal<TuiBackend>) {
        print!("{}", "\n".repeat(terminal.get_frame().size().height as usize));
    }

    pub fn draw<F>(&mut self, f: F) -> Result<(), Error>
        where F: FnOnce(&mut Frame<TuiBackend>),
    {
        self.tui_terminal.draw(f)
            .map_err(|e| Error::IOError(e))
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = self.tui_terminal.clear() {
            error!("Error clearing terminal: {}", e);
        }
    }
}

