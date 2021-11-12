use std::io;
use std::io::Stdout;

use log::error;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::layout::Rect;
use tui::widgets::{StatefulWidget, Widget};
use tui::{Frame, Terminal as TuiTerminal};

use crate::ui::Error;

pub type TuiBackend = TermionBackend<RawTerminal<Stdout>>;

pub struct Terminal {
    tui_terminal: TuiTerminal<TuiBackend>,
}

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

    pub fn draw<F>(&mut self, render_fn: F) -> Result<(), Error>
    where
        for<'f, 'g> F: FnOnce(&'f mut FrameRegion<'f, 'g>),
    {
        self.tui_terminal
            .draw(|mut frame| Self::render_on_frame(&mut frame, render_fn))
            .map(|_frame| ())
            .map_err(Error::IOError)
    }

    fn render_on_frame<F>(mut frame: &mut Frame<TuiBackend>, render_fn: F)
    where
        for<'a, 'b> F: FnOnce(&'a mut FrameRegion<'a, 'b>),
    {
        let mut frame_region = FrameRegion::new(&mut frame);
        render_fn(&mut frame_region);
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
}

impl<'a, 'b: 'a> FrameRegion<'a, 'b> {
    pub fn new(frame: &'a mut Frame<'b, TuiBackend>) -> Self {
        let region = frame.size();
        FrameRegion { frame, region }
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
}
