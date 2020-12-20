use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::error;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};

#[cfg(feature = "netio")]
use {
    spv::procfs::net_io_probe::NetIoProbe
};
use spv::core::metrics::Probe;
use spv::core::process_view::ProcessView;
use spv::procfs::cpu_probe::CpuProbe;
use spv::procfs::process::ProcfsScanner;
use spv::spv::{SpvApplication, SpvContext};
use spv::triggers::TriggersEmitter;

fn main() -> anyhow::Result<()> {
    setup_panic_logging();
    init_logging();

    let (tx, rx) = channel();

    let refresh_period = Duration::from_secs(1);
    TriggersEmitter::launch_async(tx, refresh_period);

    let app_context = build_spv_context();

    // TODO make this cleaner
    let mut probes = vec![
        Box::new(CpuProbe::new()?) as Box<dyn Probe>
    ];

    #[cfg(feature = "netio")]
        {
            probes.push(Box::new(NetIoProbe::new()?) as Box<dyn Probe>);
        }

    let app = SpvApplication::new(rx, probes, app_context, refresh_period)?;
    app.run()?;

    Ok(())
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