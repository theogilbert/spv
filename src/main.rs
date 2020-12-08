use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::error;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};

use spv::core::metrics::Probe;
use spv::core::process_view::ProcessView;
use spv::procfs::cpu_probe::CpuProbe;
use spv::procfs::net_io_probe::NetIoProbe;
use spv::procfs::process::ProcfsScanner;
use spv::spv::{SpvApplication, SpvContext};
use spv::triggers::TriggersEmitter;

fn main() {
    setup_panic_logging();
    init_logging();

    let (tx, rx) = channel();

    let probe_period = Duration::from_secs(1);
    TriggersEmitter::launch_async(tx, probe_period);

    let app_context = build_spv_context();

    let probes = vec![
        Box::new(CpuProbe::new().expect("... TODO get rid of this POC")) as Box<dyn Probe>,
        Box::new(NetIoProbe::default()) as Box<dyn Probe>];

    let app_ret = SpvApplication::new(rx, probes, app_context,
                                      probe_period);

    match app_ret {
        Err(e) => error!("{:?}", e),
        Ok(app) => {
            if let Err(e) = app.run() {
                error!("{:?}", e);
            }
        }
    };
}

fn setup_panic_logging() {
    // As panics are erased by the application exiting, log the panic as an error
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        error!("Panic occured: {:?}", info);
        default_hook(info);
    }))
}

fn init_logging() {
    let log_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("spv.log")
        .expect("Could not open log file");

    WriteLogger::init(LevelFilter::Debug, Config::default(), log_file)
        .expect("Could not initialize logging");
}


fn build_spv_context() -> SpvContext {
    let process_scanner = ProcfsScanner::new();
    let process_view = ProcessView::new(Box::new(process_scanner));

    SpvContext::new(process_view)
}