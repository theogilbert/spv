use std::thread::sleep;
use std::time::Duration;

use spv::process::{ProcessMetadata, ProcessSentry, ProcfsScanner};
use spv::probe::{CpuProbe, Probe};
use spv::metrics::{PercentValue, Value};

fn main() {
    let scanner = ProcfsScanner::new();
    let mut sentry = ProcessSentry::new(scanner);

    let mut processes = Vec::new();

    let mut cpu_probe = CpuProbe::new().expect("Could not create cpu probe");

    let on_new_process = |pm: &ProcessMetadata| {
        println!("Process {:5} spawned: {}", pm.pid(), pm.command());
        processes.push(pm.clone());
    };

    let on_process_killed = |pm: ProcessMetadata| {
        println!("Process {:5} killed: {}", pm.pid(), pm.command());
    };

    sentry.scan(on_new_process, on_process_killed)
        .expect("Could not scan processes");

    loop {
        sleep(Duration::from_secs(1));
        cpu_probe.tick();

        processes.iter().map(|pm| (pm, cpu_probe.probe(pm.pid()).ok()))
            .filter(|(_pm, cpu_usage_opt)| cpu_usage_opt.is_some())
            .for_each(|(pm, cpu_usage_opt)| {
                if let Some(cpu_usage) = cpu_usage_opt {
                    if cpu_usage.value() > 0. {
                        println!("Process {} ({}) -> {}",
                                 pm.command(), pm.pid(), cpu_usage_opt.unwrap());
                    }
                }
            });
    }
}
