use std::sync::mpsc::channel;
use std::time::Duration;

use spv::app::SpvApplication;
use spv::triggers::TriggersEmitter;

fn main() {
    let (tx, rx) = channel();

    TriggersEmitter::launch_async(tx, Duration::from_secs(1));
    let app_ret = SpvApplication::new(rx);

    match app_ret {
        Err(e) => println!("Error: {:?}", e),
        Ok(app) => {
            if let Err(e) = app.run() {
                println!("Error: {:?}", e);
            }
        }
    };
}
