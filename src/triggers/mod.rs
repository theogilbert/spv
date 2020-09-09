use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use crate::triggers::pulse::Pulse;
use crate::triggers::input::InputListener;

mod pulse;
mod input;

pub enum Error {
    InputError(String)
}

pub enum Trigger {
    Exit,
    Impulse,
}

pub struct TriggersEmitter;

impl TriggersEmitter {

    pub fn launch_async(sender: Sender<Trigger>, impulse_period: Duration) {
        let impulse_sender = sender.clone();
        let input_sender = sender.clone();

        Self::pop_impulse_thread(impulse_sender, impulse_period);
        Self::pop_input_thread(input_sender);
    }

    fn pop_impulse_thread(sender: Sender<Trigger>, impulse_period: Duration) {
        thread::spawn(move || {
            let mut pulse = Pulse::new(impulse_period);

            loop {
                if let Err(_) = sender.send(Trigger::Impulse) {
                    break
                }
                pulse.pulse();
            }
        });
    }

    fn pop_input_thread(sender: Sender<Trigger>) {
        thread::spawn(move || {
            let mut listener = InputListener::new(sender);
            listener.listen().ok();  // if an error occurs, we simply exit the thread..
        });
    }
}