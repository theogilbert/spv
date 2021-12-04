use std::sync::mpsc::Sender;
use std::time::Duration;
use std::{io, thread};

use log::error;
use thiserror::Error;

use crate::triggers::input::InputListener;
use crate::triggers::pulse::Pulse;
use crate::triggers::signal::SignalListener;

mod input;
mod pulse;
mod signal;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error reading user input")]
    InputError(#[source] io::Error),
    #[error("Error reading signal")]
    SignalError(#[source] io::Error),
}

pub enum Trigger {
    Exit,
    Impulse,
    NextProcess,
    PreviousProcess,
    Resize,
    NextTab,
    PreviousTab,
    /// Scroll the chart toward the left
    ScrollLeft,
    /// Scroll the chart toward the right
    ScrollRight,
    /// Reset the chart's default position to the current iteration
    ScrollReset,
}

pub struct TriggersEmitter;

impl TriggersEmitter {
    pub fn launch_async(sender: Sender<Trigger>, refresh_period: Duration) {
        let impulse_sender = sender.clone();
        let input_sender = sender.clone();
        let signal_sender = sender;

        Self::start_impulse_thread(impulse_sender, refresh_period);
        Self::start_input_thread(input_sender);
        Self::start_signal_thread(signal_sender);
    }

    /// Launches a thread which will emit a `Trigger::Impulse` event every `refresh_period`
    fn start_impulse_thread(sender: Sender<Trigger>, refresh_period: Duration) {
        thread::spawn(move || {
            let mut pulse = Pulse::new(refresh_period);
            loop {
                if sender.send(Trigger::Impulse).is_err() {
                    break;
                }
                pulse.pulse();
            }
        });
    }

    pub fn impulse_time_tolerance(refresh_period: Duration) -> Duration {
        Pulse::tolerance(refresh_period)
    }

    fn start_input_thread(sender: Sender<Trigger>) {
        thread::spawn(move || {
            if let Err(e) = InputListener::new(sender).listen() {
                error!("Trigger error: {:?}", e);
            }
        });
    }

    fn start_signal_thread(sender: Sender<Trigger>) {
        thread::spawn(move || {
            if let Err(e) = SignalListener::new(sender).listen() {
                error!("Trigger errors: {:?}", e);
            }
        });
    }
}
