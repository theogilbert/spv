use std::fs::OpenOptions;
use std::sync::mpsc::channel;
use std::time::Duration;

use log::error;
use log::LevelFilter;
use simplelog::{ConfigBuilder, WriteLogger};

use spv::core::collection::{MetricCollector, ProbeCollector};
use spv::core::process::ProcessCollector;
use spv::procfs::cpu_probe::CpuProbe;
use spv::procfs::diskio_probe::DiskIOProbe;
use spv::procfs::libc::open_file_limit;
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
    let impulse_tolerance = TriggersEmitter::impulse_time_tolerance(refresh_period);

    let process_scanner = ProcfsScanner::new()?;
    let process_view = ProcessCollector::new(Box::new(process_scanner));

    let collectors = build_collectors()?;

    let app = SpvApplication::new(rx, collectors, process_view, impulse_tolerance)?;
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

    let log_config = ConfigBuilder::default().set_time_format_rfc2822().build();

    WriteLogger::init(LevelFilter::Debug, log_config, log_file).expect("Could not initialize logging");
}

fn build_collectors() -> Result<Vec<Box<dyn MetricCollector>>, Error> {
    let fd_not_for_probes = 10; // ~ the no of files that the application will keep open not for probing purposes
    let max_fd = open_file_limit().expect("Could not read process file limits") as usize - fd_not_for_probes;

    let mut collectors = vec![];

    let cpu_probe = CpuProbe::new(max_fd / 2).map_err(Error::CoreError)?;
    let cpu_collector = ProbeCollector::new(cpu_probe);
    collectors.push(Box::new(cpu_collector) as Box<dyn MetricCollector>);

    let disk_io_probe = DiskIOProbe::new(max_fd / 2);
    let disk_io_collector = ProbeCollector::new(disk_io_probe);
    collectors.push(Box::new(disk_io_collector) as Box<dyn MetricCollector>);

    #[cfg(feature = "netio")]
    {
        let netio_probe = NetIoProbe::new().map_err(Error::CoreError)?;
        let net_collector = ProbeCollector::new(netio_probe);
        collectors.push(Box::new(net_collector) as Box<dyn MetricCollector>);
    }

    Ok(collectors)
}
