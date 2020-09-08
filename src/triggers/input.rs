use std::io::stdin;
use std::sync::mpsc::Sender;

use termion::event::Key;
use termion::input::TermRead;

use crate::triggers::{Error, Trigger};

pub struct InputListener {
    sender: Sender<Trigger>,
    exit: bool,
}

impl InputListener {
    pub fn new(sender: Sender<Trigger>) -> Self {
        Self { sender, exit: false }
    }

    pub fn listen(mut self) -> Result<(), Error> {
        let stdin = stdin();

        for key_ret in stdin.keys() {
            let key = key_ret.map_err(|e| Error::InputError(e.to_string()))?;

            match key {
                Key::Ctrl(c) => {
                    if c == 'c' || c == 'd' {
                        self.send(Trigger::Exit);
                        break;
                    }
                }
                _ => ()
            }

            if self.exit {
                break
            }
        }

        Ok(())
    }

    fn send(&mut self, trigger: Trigger) {
        if let Err(_) = self.sender.send(trigger) {
            self.exit = true;
        }
    }
}