use std::sync::mpsc::channel;
use std::time::Duration;

use spv::app::{SpvApplication, SpvContext};
use spv::triggers::TriggersEmitter;
use spv::core::process_view::ProcessView;
use spv::procfs::process::ProcfsScanner;

fn main() {
    let (tx, rx) = channel();

    TriggersEmitter::launch_async(tx, Duration::from_secs(1));

    let app_context = build_spv_context();
    let app_ret = SpvApplication::new(rx, app_context);

    match app_ret {
        Err(e) => println!("Error: {:?}", e),
        Ok(app) => {
            if let Err(e) = app.run() {
                println!("Error: {:?}", e);
            }
        }
    };
}


fn build_spv_context() -> SpvContext {
    let process_scanner = ProcfsScanner::new();
    let process_view = ProcessView::new(Box::new(process_scanner));

    SpvContext::new(process_view)
}