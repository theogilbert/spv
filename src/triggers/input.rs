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
            let key = key_ret.map_err(Error::InputError)?;

            match key {
                Key::Ctrl(c) => self.on_ctrl_key_pressed(c),
                Key::Char(c) => self.on_key_pressed(c),
                Key::Left => self.send(Trigger::PreviousTab),
                Key::Right => self.send(Trigger::NextTab),
                Key::Up => self.send(Trigger::PreviousProcess),
                Key::Down => self.send(Trigger::NextProcess),
                _ => ()
            }

            if self.exit {
                break;
            }
        }

        Ok(())
    }

    fn on_ctrl_key_pressed(&mut self, key: char) {
        match key {
            'c' => self.send_exit(),
            'd' => self.send_exit(),
            _ => ()
        }
    }

    fn on_key_pressed(&mut self, key: char) {
        if let 'q' = key {
            self.send_exit();
        }
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