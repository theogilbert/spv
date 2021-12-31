use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use rstest::{fixture, rstest};

use spv::core::probe::Probe;
use spv::core::process::ProcessCollector;
use spv::procfs::cpu_probe::CpuProbe;
use spv::procfs::process::ProcfsScanner;

#[fixture]
fn collector() -> ProcessCollector {
    let process_scanner = ProcfsScanner::new().expect("Could not create procfs scanner");
    ProcessCollector::new(Box::new(process_scanner))
}

#[fixture]
fn probe() -> CpuProbe {
    CpuProbe::new(1000).expect("Could not create cpu probe")
}

#[rstest]
fn test_should_not_fail_due_to_too_many_open_files_over_time(mut collector: ProcessCollector, mut probe: CpuProbe) {
    (0..2).for_each(|_| {
        spawn_processes(500, "sleep 0.1");
        collector.collect_processes().expect("Could not collect processes");
        probe
            .probe_processes(&collector.running_pids())
            .expect("Error running processes");
        sleep(Duration::from_millis(100));
    });
}

#[rstest]
fn test_should_not_fail_due_to_too_many_open_files_at_once(mut collector: ProcessCollector, mut probe: CpuProbe) {
    spawn_processes(1000, "sleep 1");

    collector.collect_processes().expect("Could not collect processes");

    probe
        .probe_processes(&collector.running_pids())
        .expect("Error running processes");
}

fn spawn_processes(count: usize, cmd: &str) {
    (0..count).for_each(|_| {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .expect("Could not launch child process");
    })
}
