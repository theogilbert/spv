use std::io;
use std::io::Stdout;

use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::TermionBackend;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::Terminal;
use tui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::probe::Frame;
use crate::probe::process::{PID, ProcessMetadata};

pub struct FrameRenderer {
    terminal: Terminal<TermionBackend<RawTerminal<Stdout>>>,
}

impl FrameRenderer {
    pub fn new() -> Result<FrameRenderer, io::Error> {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(FrameRenderer { terminal })
    }

    pub fn render(&mut self, frame: Frame) {
        println!("Rendering frame..");
    }
}
