use std::sync::mpsc::Sender;

use libc::c_int;
use signal_hook::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::triggers::{Error, Trigger};

pub struct SignalListener {
    sender: Sender<Trigger>,
    exit: bool,
}

impl SignalListener {
    pub fn new(sender: Sender<Trigger>) -> Self {
        Self { sender, exit: false }
    }

    pub fn listen(mut self) -> Result<(), Error> {
        let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT])
            .map_err(|e| Error::SignalError(e.to_string()))?;

        while !self.exit {
            for signal in signals.wait() {
                match signal as c_int {
                    signal_hook::SIGTERM | signal_hook::SIGINT | signal_hook::SIGQUIT => {
                        self.send_exit();
                    }
                    _ => unreachable!()
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