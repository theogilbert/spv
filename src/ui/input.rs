use std::io::{Stdin, stdin};
use termion::event::Key;
use termion::input::{TermRead, Keys};
use crate::ui::Error;

pub struct InputListener {
    keys_iterator: Keys<Stdin>
}

pub enum Input {
    Exit,
    None,
}

impl InputListener {
    pub fn new() -> Self {
        Self { keys_iterator: stdin().keys() }
    }

    pub fn listen(&mut self) -> Result<Input, Error> {
        let key_opt = self.keys_iterator.next();

        if let Some(key_ret) = key_opt {
            let key = key_ret.map_err(|e| Error::InputError(e.to_string()))?;
            return match key {
                Key::Ctrl(c) => {
                    if c == 'c' || c == 'd' {
                        Ok(Input::Exit)
                    } else {
                        Ok(Input::None)
                    }
                }
                _ => Ok(Input::None)
            }
        }

        Ok(Input::None)
    }
}