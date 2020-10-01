use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use simplelog::{WriteLogger, Config};
use log::error;

use spv::app::{SpvApplication, SpvContext};
use spv::core::process_view::ProcessView;
use spv::procfs::process::ProcfsScanner;
use spv::triggers::TriggersEmitter;
use std::env;
use log::LevelFilter;

fn main() {
    init_logging();

    let (tx, rx) = channel();

    TriggersEmitter::launch_async(tx, Duration::from_secs(3));

    let app_context = build_spv_context();
    let app_ret = SpvApplication::new(rx, app_context);

    match app_ret {
        Err(e) => error!("{:?}", e),
        Ok(app) => {
            if let Err(e) = app.run() {
                error!("{:?}", e);
            }
        }
    };
}

fn init_logging() {
    let log_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("spv.log")
        .expect("Could not open log file");

    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file);
}


fn build_spv_context() -> SpvContext {
    let process_scanner = ProcfsScanner::new();
    let process_view = ProcessView::new(Box::new(process_scanner));

    SpvContext::new(process_view)
}