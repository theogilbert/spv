use std::io::stdin;
use std::sync::mpsc::Sender;

use termion::event::Key as TermionKey;
use termion::input::TermRead;

use crate::triggers::{Error, Input, Trigger};

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
                TermionKey::Ctrl(c) => self.on_ctrl_key_pressed(c),
                TermionKey::Char(c) => self.on_key_pressed(c),
                TermionKey::Left => self.send(Trigger::Input(Input::Left)),
                TermionKey::Right => self.send(Trigger::Input(Input::Right)),
                TermionKey::Up => self.send(Trigger::Input(Input::Up)),
                TermionKey::Down => self.send(Trigger::Input(Input::Down)),
                TermionKey::Esc => self.send(Trigger::Input(Input::Escape)),
                _ => (),
            }

            if self.exit {
                break;
            }
        }

        Ok(())
    }

    fn on_ctrl_key_pressed(&mut self, key: char) {
        match key {
            'c' | 'd' => self.send_exit(),
            _ => (),
        }
    }

    fn on_key_pressed(&mut self, key: char) {
        match key {
            'q' => self.send_exit(),
            'h' => self.send(Trigger::Input(Input::Left)),
            'j' => self.send(Trigger::Input(Input::Down)),
            'k' => self.send(Trigger::Input(Input::Up)),
            'l' => self.send(Trigger::Input(Input::Right)),
            'H' => self.send(Trigger::Input(Input::AltLeft)),
            'J' => self.send(Trigger::Input(Input::AltDown)),
            'K' => self.send(Trigger::Input(Input::AltUp)),
            'L' => self.send(Trigger::Input(Input::AltRight)),
            'g' => self.send(Trigger::Input(Input::G)),
            's' => self.send(Trigger::Input(Input::S)),
            '\n' => self.send(Trigger::Input(Input::Submit)),
            _ => {}
        };
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
