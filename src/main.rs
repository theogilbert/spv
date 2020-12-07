use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::error;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};

use spv::core::metrics::Probe;
use spv::core::process_view::ProcessView;
use spv::procfs::cpu_probe::CpuProbe;
use spv::procfs::process::ProcfsScanner;
use spv::spv::{SpvApplication, SpvContext};
use spv::triggers::TriggersEmitter;

fn main() {
    setup_panic();
    init_logging();

    let (tx, rx) = channel();

    let probe_period = Duration::from_secs(1);
    TriggersEmitter::launch_async(tx, probe_period);

    let app_context = build_spv_context();

    let probes = vec![Box::new(CpuProbe::new().expect("... TODO get rid of this POC")) as Box<dyn Probe>];

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

fn setup_panic() {
    // As panic! is swallowed by the raw terminal, log the panic as well
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let payload = match info.payload().downcast_ref::<&str>() {
            Some(c) => *c,
            None => "",
        };

        let formatted_location = match info.location() {
            None => "Could not retrieve panic location".to_string(),
            Some(loc) => format!("Panic occured in file '{}' at line '{}'",
                                 loc.file(), loc.line()),
        };

        error!("Panic occured '{}'. {}", payload, formatted_location);

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