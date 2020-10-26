//! CPU Usage probing

use std::collections::HashMap;

use crate::core::Error;
use crate::core::metrics::{Metric, Probe};
use crate::core::process_view::PID;
use crate::core::values::Percent;
use crate::procfs::parsers;
use crate::procfs::parsers::{PidStat, ProcessDataReader, ReadProcessData, ReadSystemData, Stat, SystemDataReader};

/// Probe implementation to measure the CPU usage (in percent) of processes
pub struct CpuProbe {
    stat_reader: Box<dyn ReadSystemData<Stat>>,
    pid_stat_reader: Box<dyn ReadProcessData<PidStat>>,
    calculator: UsageCalculator,
}

impl CpuProbe {
    pub fn new() -> Result<Self, Error> {
        let stat_reader = SystemDataReader::new()
            .map_err(|e| {
                Error::ProbingError("Error initializing SystemDataReader".to_string(), Box::new(e))
            })?;

        Self::from_readers(Box::new(stat_reader), Box::new(ProcessDataReader::new()))
    }

    pub fn from_readers(mut stat_reader: Box<dyn ReadSystemData<Stat>>,
                        pid_stat_reader: Box<dyn ReadProcessData<PidStat>>) -> Result<Self, Error> {
        let stat_data = stat_reader.read()
            .map_err(|e| Error::ProbingError("Error initializing CPU Probe".to_string(),
                                             Box::new(e)))?;

        Ok(CpuProbe {
            pid_stat_reader,
            stat_reader,
            calculator: UsageCalculator::new(stat_data),
        })
    }
}

impl Probe for CpuProbe {
    fn name(&self) -> &'static str {
        "CPU usage"
    }

    fn default_metric(&self) -> Metric {
        Metric::from_percent(f32::NAN).unwrap()
    }

    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_reader
            .read()
            .map_err(|e| {
                Error::ProbingError("Error reading global CPU stats".to_string(), Box::new(e))
            })?;

        self.calculator.update_stat_data(new_stat);

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<Metric, Error> {
        let pid_stat = self.pid_stat_reader
            .read(pid)
            .map_err(|e| {
                Error::ProbingError(format!("Error probing CPU stats for PID {}", pid),
                                    Box::new(e))
            })?;

        let percent = self.calculator.calculate_pid_usage(pid, pid_stat)?;
        Ok(Metric::Percent(percent))
    }
}

#[cfg(test)]
mod test_cpu_probe {
    use std::collections::HashMap;
    use std::io;

    use crate::core::metrics::{Metric, Probe};
    use crate::core::process_view::PID;
    use crate::procfs::cpu_probe::common_test_utils::{create_pid_stat, create_stat};
    use crate::procfs::cpu_probe::CpuProbe;
    use crate::procfs::parsers::{PidStat, ReadProcessData, ReadSystemData, Stat};
    use crate::procfs::ProcfsError;

    struct MemoryPidStatReader {
        pid_stats_seq: HashMap<PID, Result<PidStat, ProcfsError>>
    }

    impl ReadProcessData<PidStat> for MemoryPidStatReader {
        fn read(&mut self, pid: u32) -> Result<PidStat, ProcfsError> {
            self.pid_stats_seq.remove(&pid)
                .unwrap()
        }
    }

    struct InMemoryStatReader {
        stat_seq: Vec<Stat>
    }

    impl ReadSystemData<Stat> for InMemoryStatReader {
        fn read(&mut self) -> Result<Stat, ProcfsError> {
            Ok(self.stat_seq.remove(0))
        }
    }

    #[test]
    fn test_should_return_zero_metrics_when_no_pid() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };
        let pid_stat_reader = MemoryPidStatReader { pid_stats_seq: HashMap::new() };

        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        let empty_map: HashMap<PID, Metric> = HashMap::new();
        assert!(matches!(probe.probe_processes(&vec![]), Ok(empty_map)));
    }


    #[test]
    fn test_should_return_one_metric_when_one_pid() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => Ok(create_pid_stat(100)))
        };

        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        assert_eq!(probe.probe_processes(&vec![1]).unwrap(),
                   hashmap!(1 => Metric::from_percent(50.).unwrap()));
    }


    #[test]
    fn test_should_return_two_metrics_when_two_pids() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => Ok(create_pid_stat(50)), 2 => Ok(create_pid_stat(50)))
        };

        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        let metrics = probe.probe_processes(&vec!(1, 2)).unwrap();
        assert_eq!(metrics,
                   hashmap!(1 => Metric::from_percent(25.).unwrap(),
                   2 => Metric::from_percent(25.).unwrap()));
    }


    #[test]
    fn test_should_return_ignore_pid_when_probe_returns_err() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(
                1 => Ok(create_pid_stat(50)),
                2 => Err(ProcfsError::IoError(io::Error::new(io::ErrorKind::Other, "oh no!")))
            )
        };


        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        let map = hashmap!(1 => Metric::from_percent(25.).unwrap());
        assert!(matches!(probe.probe_processes(&vec![1, 2]), Ok(map)));
    }
}

struct UsageCalculator {
    processes_prev_stats: HashMap<PID, parsers::PidStat>,
    prev_global_stat: parsers::Stat,
    global_runtime_diff: f64,
}

impl UsageCalculator {
    pub fn new(init_stat_data: parsers::Stat) -> Self {
        UsageCalculator {
            processes_prev_stats: HashMap::new(),
            prev_global_stat: init_stat_data,
            global_runtime_diff: 0.,
        }
    }

    pub fn update_stat_data(&mut self, stat_data: Stat) {
        let cur_runtime = stat_data.running_time();
        let prev_runtime = self.prev_global_stat.running_time();

        self.global_runtime_diff = (cur_runtime - prev_runtime) as f64;
        self.prev_global_stat = stat_data;
    }

    pub fn calculate_pid_usage(&mut self, pid: PID, pid_stat_data: PidStat)
                               -> Result<Percent, Error> {
        let last_iter_runtime = match self.processes_prev_stats.get(&pid) {
            Some(stat_data) => stat_data.running_time(),
            None => 0
        };

        let pid_runtime_diff = pid_stat_data.running_time() - last_iter_runtime;
        self.processes_prev_stats.insert(pid, pid_stat_data);

        let ratio = pid_runtime_diff as f64 / self.global_runtime_diff;
        let percent = (100. * ratio) as f32;

        Percent::new(percent)
            .map_err(|e| {
                Error::ProbingError(format!("Invalid CPU usage value for PID {} : {}",
                                            pid, percent), Box::new(e))
            })
    }
}


#[cfg(test)]
mod test_cpu_calculator {
    use crate::core::values::Percent;
    use crate::procfs::cpu_probe::common_test_utils::create_stat;
    use crate::procfs::cpu_probe::UsageCalculator;
    use crate::procfs::parsers;

    fn create_initialized_calc(elapsed_ticks: u64) -> UsageCalculator {
        let mut calc = UsageCalculator::new(create_stat(100));

        calc.update_stat_data(create_stat(100 + elapsed_ticks));

        calc
    }

    #[test]
    fn test_zero_percent_usage() {
        let mut calc = create_initialized_calc(60);

        let pid_stat = parsers::PidStat::new(0, 0, 0, 0);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat).unwrap(),
                   Percent::new(0.).unwrap());
    }

    #[test]
    fn test_hundred_percent_usage() {
        let mut calc = create_initialized_calc(123);

        let pid_stat = parsers::PidStat::new(100, 20, 2, 1);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat).unwrap(),
                   Percent::new(100.).unwrap());
    }

    #[test]
    fn test_over_hundred_percent_usage() {
        let mut calc = create_initialized_calc(20);

        let pid_stat = parsers::PidStat::new(10, 10, 10, 10);

        // 40 ticks spent by pid, but only 20 by cpu -> 200% cpu usage
        assert!(calc.calculate_pid_usage(1, pid_stat).is_err());
    }
}

#[cfg(test)]
mod common_test_utils {
    use crate::procfs::parsers::{PidStat, Stat};

    pub fn create_stat(running_time: u64) -> Stat {
        let individual_ticks = running_time / 6;
        let leftover = running_time - 6 * individual_ticks;

        Stat::new(individual_ticks, individual_ticks, individual_ticks,
                  individual_ticks, individual_ticks,
                  individual_ticks + leftover)
    }

    pub fn create_pid_stat(running_time: u32) -> PidStat {
        let individual_ticks = (running_time / 4) as u32;
        let leftover = (running_time - 4 * individual_ticks) as u32;

        PidStat::new(individual_ticks, individual_ticks,
                     individual_ticks as i32, (individual_ticks + leftover) as i32)
    }
}