use log::error;
use tui::layout::Rect;
use tui::widgets::{StatefulWidget, Widget};
use tui::{Frame, Terminal as TuiTerminal};

#[cfg(not(test))]
use {
    std::io,
    std::io::Stdout,
    termion::raw::{IntoRawMode, RawTerminal},
    tui::backend::TermionBackend,
};
#[cfg(test)]
use {tui::backend::TestBackend, tui::buffer::Buffer};

use crate::ui::Error;

#[cfg(not(test))]
pub type TuiBackend = TermionBackend<RawTerminal<Stdout>>;
#[cfg(test)]
pub type TuiBackend = TestBackend;

pub struct Terminal {
    tui_terminal: TuiTerminal<TuiBackend>,
}

#[cfg(not(test))]
impl Terminal {
    pub fn new() -> Result<Self, Error> {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);

        let mut tui_terminal = TuiTerminal::new(backend)?;

        Self::generate_empty_frame(&mut tui_terminal);

        Ok(Terminal { tui_terminal })
    }

    /// On application startup, calling self.clear() would overwrite the current content of the
    /// terminal. By generating an empty frame instead and then overwriting that frame, we do not
    /// erase any existing content on application startup.
    fn generate_empty_frame(terminal: &mut TuiTerminal<TuiBackend>) {
        print!("{}", "\n".repeat(terminal.get_frame().size().height as usize));
    }
}

#[cfg(test)]
impl Terminal {
    pub fn new() -> Result<Self, Error> {
        Self::from_size(200, 100)
    }

    pub fn from_size(width: u16, height: u16) -> Result<Self, Error> {
        let backend = TestBackend::new(width, height);

        let tui_terminal = TuiTerminal::new(backend)?;

        Ok(Terminal { tui_terminal })
    }

    pub fn assert_buffer(&self, buffer: Buffer) {
        self.tui_terminal.backend().assert_buffer(&buffer);
    }
}

impl Terminal {
    pub fn draw<F>(&mut self, render_fn: F) -> Result<(), Error>
    where
        for<'f, 'g> F: FnOnce(&'f mut FrameRegion<'f, 'g>),
    {
        self.tui_terminal
            .draw(|frame| {
                let mut frame_region = FrameRegion::new(frame);
                render_fn(&mut frame_region);
            })
            .map(|_frame| ())
            .map_err(Error::IOError)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(e) = self.tui_terminal.clear() {
            error!("Error clearing terminal: {}", e);
        }
    }
}

pub struct FrameRegion<'a, 'b: 'a> {
    frame: &'a mut Frame<'b, TuiBackend>,
    region: Rect,
    original_region: Rect,
}

impl<'a, 'b: 'a> FrameRegion<'a, 'b> {
    pub fn new(frame: &'a mut Frame<'b, TuiBackend>) -> Self {
        let region = frame.size();
        FrameRegion {
            frame,
            region,
            original_region: region,
        }
    }
    pub fn render_widget<W>(&mut self, widget: W)
    where
        W: Widget,
    {
        self.frame.render_widget(widget, self.region);
    }

    pub fn render_stateful_widget<W>(&mut self, widget: W, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        self.frame.render_stateful_widget(widget, self.region, state);
    }

    pub fn region(&self) -> Rect {
        self.region
    }

    pub fn with_region(&mut self, region: Rect) -> &mut Self {
        self.region = region;
        self
    }

    pub fn with_original_region(&mut self) -> &mut Self {
        self.region = self.original_region;
        self
    }
}
