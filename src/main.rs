use std::io;
use std::sync::mpsc::channel;
use std::time::Duration;

use spv::app::{SpvApplication, SpvContext};
use spv::triggers::TriggersEmitter;

fn main() {
    let (tx, rx) = channel();

    TriggersEmitter::launch_async(tx, Duration::from_secs(1));
    SpvApplication::new(rx, SpvContext::new())
        .run();
}
