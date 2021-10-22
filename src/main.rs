use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::error;
use log::LevelFilter;
use simplelog::{ConfigBuilder, WriteLogger};

use spv::core::collection::{MetricCollector, ProbeCollector};
use spv::core::process::ProcessCollector;
use spv::procfs::cpu_probe::CpuProbe;
#[cfg(feature = "netio")]
use spv::procfs::net_io_probe::NetIoProbe;
use spv::procfs::process::ProcfsScanner;
use spv::spv::SpvApplication;
use spv::triggers::TriggersEmitter;
use spv::Error;

fn main() -> anyhow::Result<()> {
    setup_panic_logging();
    init_logging();

    let (tx, rx) = channel();

    let refresh_period = Duration::from_secs(1);
    TriggersEmitter::launch_async(tx, refresh_period);

    let process_scanner = ProcfsScanner::new();
    let process_view = ProcessCollector::new(Box::new(process_scanner));

    let collectors = build_collectors(refresh_period)?;

    let app = SpvApplication::new(rx, collectors, process_view)?;
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

    let log_config = ConfigBuilder::default()
        .set_time_format_str("%Y-%m-%d %H:%M:%S%.3f")
        .build();

    WriteLogger::init(LevelFilter::Debug, log_config, log_file).expect("Could not initialize logging");
}

fn build_collectors(resolution: Duration) -> Result<Vec<Box<dyn MetricCollector>>, Error> {
    let mut collectors = vec![];

    let cpu_probe = CpuProbe::new().map_err(Error::CoreError)?;
    let cpu_collector = ProbeCollector::new(cpu_probe, resolution);
    collectors.push(Box::new(cpu_collector) as Box<dyn MetricCollector>);

    #[cfg(feature = "netio")]
    {
        let netio_probe = NetIoProbe::new().map_err(Error::CoreError)?;
        let net_collector = ProbeCollector::new(netio_probe, resolution);
        collectors.push(Box::new(net_collector) as Box<dyn MetricCollector>);
    }

    Ok(collectors)
}
