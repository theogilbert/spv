use std::io;
use std::sync::mpsc::{Receiver, Sender, RecvError};
use std::time::Duration;

use crate::ui::FrameRenderer;
use crate::probe::Frame;


pub enum Command {
    Refresh(Frame),
    Kill,
}


pub struct ProbingThread {
    transmitter: Sender<Command>,
    cadency: Duration,
}

impl ProbingThread {
    pub fn new(tx: Sender<Command>, cadency: Duration) -> ProbingThread {
        ProbingThread { transmitter: tx, cadency }
    }

    pub fn run() {
        loop {}
    }
}


pub struct RenderingThread {
    command_receiver: Receiver<Command>,
    frame_renderer: FrameRenderer,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    MPSCError(RecvError),
}

impl RenderingThread {
    pub fn new(command_receiver: Receiver<Command>) -> Result<RenderingThread, io::Error> {
        let renderer = FrameRenderer::new()?;

        Ok(RenderingThread { command_receiver, frame_renderer: renderer })
    }

    pub fn run(mut self) -> Result<(), Error> {
        loop {
            match self.command_receiver.recv() {
                Ok(cmd) => {
                    match cmd {
                        Command::Refresh(frame) => {
                            self.frame_renderer.render(frame)
                        },
                        Command::Kill => break
                    }
                }
                Err(err) => Err(Error::MPSCError(err))?
            }
        };

        Ok(())
    }
}