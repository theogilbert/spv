use std::sync::mpsc::Sender;

use signal_hook::iterator::Signals;

use crate::triggers::{Error, Trigger};
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM, SIGWINCH};

pub struct SignalListener {
    sender: Sender<Trigger>,
    exit: bool,
}

/// Listens for UNIX interrupt signals and emits appropriate triggers
impl SignalListener {
    pub fn new(sender: Sender<Trigger>) -> Self {
        Self { sender, exit: false }
    }

    pub fn listen(mut self) -> Result<(), Error> {
        let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT, SIGWINCH]).map_err(Error::SignalError)?;

        while !self.exit {
            for signal in signals.wait() {
                match signal as i32 {
                    SIGTERM | SIGINT | SIGQUIT => {
                        self.send_exit();
                    }
                    SIGWINCH => self.send(Trigger::Resize),
                    _ => unreachable!(),
                }
            }
        }

        Ok(())
    }

    fn send_exit(&mut self) {
        self.send(Trigger::Exit);
        self.exit = true;
    }

    fn send(&mut self, trigger: Trigger) {
        if self.sender.send(trigger).is_err() {
            self.exit = true;
        }
    }
}
