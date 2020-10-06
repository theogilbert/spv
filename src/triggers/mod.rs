use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use log::error;

use crate::triggers::input::InputListener;
use crate::triggers::pulse::Pulse;
use crate::triggers::signal::SignalListener;
use std::fmt::{Display, Formatter};
use crate::fmt;

mod pulse;
mod input;
mod signal;

pub enum Error {
    InputError(String),
    SignalError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::InputError(s) => write!(f, "InputError: {}", s),
            Error::SignalError(s) => write!(f, "SignalError: {}", s),
        }
    }
}

pub enum Trigger {
    Exit,
    Impulse,
    NextProcess,
    PreviousProcess,
}

pub struct TriggersEmitter;

// TODO add signal hook for SIGWINCH (terminal resize) to ask to redraw
impl TriggersEmitter {
    pub fn launch_async(sender: Sender<Trigger>, impulse_period: Duration) {
        let impulse_sender = sender.clone();
        let input_sender = sender.clone();
        let signal_sender = sender;

        Self::start_impulse_thread(impulse_sender, impulse_period);
        Self::start_input_thread(input_sender);
        Self::start_signal_thread(signal_sender);
    }

    fn start_impulse_thread(sender: Sender<Trigger>, impulse_period: Duration) {
        thread::spawn(move || {
            let mut pulse = Pulse::new(impulse_period);

            loop {
                if sender.send(Trigger::Impulse).is_err() {
                    break;
                }
                pulse.pulse();
            }
        });
    }

    fn start_input_thread(sender: Sender<Trigger>) {
        thread::spawn(move || {
            if let Err(e) = InputListener::new(sender).listen() {
                error!("Trigger error: {}", e);
            }
        });
    }

    fn start_signal_thread(sender: Sender<Trigger>) {
        thread::spawn(move || {
            if let Err(e) = SignalListener::new(sender).listen() {
                error!("Trigger errors: {}", e);
            }
        });
    }
}