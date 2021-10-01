//! CPU Usage probing

use std::collections::HashMap;

use crate::core::Error;
use crate::core::metrics::PercentMetric;
use crate::core::probe::Probe;
use crate::core::process_view::Pid;
use crate::procfs::parsers;
use crate::procfs::parsers::{PidStat, ProcessDataReader, ReadProcessData, ReadSystemData, Stat, SystemDataReader};

// TODO When a process CPU usage is low, some iterations will detect a CPU usage of 0%, causing a
//   fluctuating value between each iterations. Fix this, maybe by averaging reported values over
//   last N probed iterations

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
                Error::ProbingError("Error initializing SystemDataReader".to_string(), e.into())
            })?;


        Self::from_readers(Box::new(stat_reader), Box::new(ProcessDataReader::new()))
    }

    fn from_readers(stat_reader: Box<dyn ReadSystemData<Stat>>,
                    pid_stat_reader: Box<dyn ReadProcessData<PidStat>>) -> Result<Self, Error> {
        Ok(CpuProbe {
            pid_stat_reader,
            stat_reader,
            calculator: UsageCalculator::default(),
        })
    }
}

impl Probe<PercentMetric> for CpuProbe {
    fn name(&self) -> &'static str {
        "CPU usage"
    }

    fn default_metric(&self) -> PercentMetric {
        PercentMetric::new(0.)
    }

    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_reader
            .read()
            .map_err(|e| {
                Error::ProbingError("Error reading global CPU stats".to_string(), e.into())
            })?;

        self.calculator.compute_new_runtime_diff(new_stat);

        Ok(())
    }

    fn probe(&mut self, pid: Pid) -> Result<PercentMetric, Error> {
        let pid_stat = self.pid_stat_reader
            .read(pid)
            .map_err(|e| {
                Error::ProbingError(format!("Error probing CPU stats for PID {}", pid),
                                    e.into())
            })?;

        let percent = self.calculator.calculate_pid_usage(pid, pid_stat);
        Ok(PercentMetric::new(percent))
    }
}


struct UsageCalculator {
    processes_prev_stats: HashMap<Pid, parsers::PidStat>,
    prev_global_stat: parsers::Stat,
    global_runtime_diff: f64,
}

impl Default for UsageCalculator {
    fn default() -> Self {
        UsageCalculator {
            processes_prev_stats: HashMap::new(),
            prev_global_stat: parsers::Stat::new(0, 0, 0, 0, 0, 0),
            global_runtime_diff: 0.,
        }
    }
}

impl UsageCalculator {
    ///
    /// Given new content of /proc/stat and the last known content of /proc/stat, calculates the
    /// elapsed ticks corresponding to global CPU runtime in this lapse of time
    ///
    /// # Arguments
    ///  * `stat_data` The new content of /proc/stat
    ///
    pub fn compute_new_runtime_diff(&mut self, stat_data: Stat) {
        let cur_runtime = stat_data.running_time();
        let prev_runtime = self.prev_global_stat.running_time();

        self.global_runtime_diff = (cur_runtime - prev_runtime) as f64;
        self.prev_global_stat = stat_data;
    }

    ///
    /// Given new content of /proc/[pid]/stat and its last known content, calculates the elapsed
    /// ticks corresponding to CPU runtime related to this process
    ///
    /// Then given a recently calculated global CPU runtime lapse (see [compute_new_runtime_diff()]),
    /// calculates the portion of this runtime that was dedicated to the given process in percent
    ///
    /// # Arguments
    ///  * `pid` The ID of a process
    ///  * `pid_stat_data`: The new content of the stat file of the process with ID [pid]
    ///
    pub fn calculate_pid_usage(&mut self, pid: Pid, pid_stat_data: PidStat) -> f64 {
        let last_iter_runtime = match self.processes_prev_stats.get(&pid) {
            Some(stat_data) => stat_data.running_time(),
            None => 0
        };

        let pid_runtime_diff = pid_stat_data.running_time() - last_iter_runtime;
        self.processes_prev_stats.insert(pid, pid_stat_data);

        100. * pid_runtime_diff as f64 / self.global_runtime_diff
    }
}


#[cfg(test)]
mod test_cpu_probe {
    use std::collections::{HashMap, VecDeque};
    use std::io;

    use crate::core::metrics::{Metric, PercentMetric, Probe};
    use crate::core::probe::Probe;
    use crate::core::process_view::Pid;
    use crate::procfs::cpu_probe::common_test_utils::{create_pid_stat, create_stat};
    use crate::procfs::cpu_probe::CpuProbe;
    use crate::procfs::parsers::{PidStat, ReadProcessData, ReadSystemData, Stat};
    use crate::procfs::ProcfsError;

    struct MemoryPidStatReader {
        pid_stats_seq: HashMap<Pid, VecDeque<Result<PidStat, ProcfsError>>>,
    }

    impl ReadProcessData<PidStat> for MemoryPidStatReader {
        fn read(&mut self, pid: u32) -> Result<PidStat, ProcfsError> {
            self.pid_stats_seq.get_mut(&pid)
                .unwrap()
                .pop_front()
                .unwrap()
        }
    }

    struct InMemoryStatReader {
        stat_seq: Vec<Stat>,
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

        let empty_map: HashMap<Pid, PercentMetric> = HashMap::new();
        assert!(matches!(probe.probe_processes(&vec![]), Ok(empty_map)));
    }


    #[test]
    fn test_should_return_one_metric_when_one_pid() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };

        let pid_stat_seq = vecdeque!(Ok(create_pid_stat(0)), Ok(create_pid_stat(100)));
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => pid_stat_seq)
        };

        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        probe.probe_processes(&vec![1]); // First calibration probing

        assert_eq!(probe.probe_processes(&vec![1]).unwrap(),
                   hashmap!(1 => PercentMetric::new(50.)));
    }


    #[test]
    fn test_should_return_two_metrics_when_two_pids() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };

        let first_pid_stat_seq = vecdeque!(Ok(create_pid_stat(0)), Ok(create_pid_stat(50)));
        let second_pid_stat_seq = vecdeque!(Ok(create_pid_stat(0)), Ok(create_pid_stat(50)));
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => first_pid_stat_seq, 2 => second_pid_stat_seq)
        };

        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");
        probe.probe_processes(&vec!(1, 2)); // calibrating probe

        let metrics = probe.probe_processes(&vec!(1, 2)).unwrap();
        assert_eq!(metrics,
                   hashmap!(1 => PercentMetric::new(25.), 2 => PercentMetric::new(25.)));
    }


    #[test]
    fn test_should_return_ignore_pid_when_probe_returns_err() {
        let stat_reader = InMemoryStatReader {
            stat_seq: vec![create_stat(0), create_stat(200)]
        };
        let first_pid_stat_seq = vecdeque!(Ok(create_pid_stat(0)), Ok(create_pid_stat(50)));
        let second_pid_stat_seq = vecdeque!(Ok(create_pid_stat(0)),
            Err(ProcfsError::IOError(io::Error::new(io::ErrorKind::Other, "oh no!"))));
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => first_pid_stat_seq, 2 => second_pid_stat_seq)
        };


        let mut probe = CpuProbe::from_readers(Box::new(stat_reader),
                                               Box::new(pid_stat_reader))
            .expect("Could not create procfs");

        let map = hashmap!(1 => PercentMetric::new(25.));
        assert!(matches!(probe.probe_processes(&vec![1, 2]), Ok(map)));
    }
}


#[cfg(test)]
mod test_cpu_calculator {
    use crate::procfs::cpu_probe::common_test_utils::create_stat;
    use crate::procfs::cpu_probe::UsageCalculator;
    use crate::procfs::parsers;

    fn create_initialized_calc(elapsed_ticks: u64) -> UsageCalculator {
        let mut calc = UsageCalculator::default();

        calc.compute_new_runtime_diff(create_stat(100));
        calc.compute_new_runtime_diff(create_stat(100 + elapsed_ticks));

        calc
    }

    #[test]
    fn test_zero_percent_usage() {
        let mut calc = create_initialized_calc(60);

        let pid_stat = parsers::PidStat::new(0, 0, 0, 0);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat),
                   0.);
    }

    #[test]
    fn test_hundred_percent_usage() {
        let mut calc = create_initialized_calc(123);

        let pid_stat = parsers::PidStat::new(100, 20, 2, 1);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat),
                   100.);
    }
}

#[cfg(test)]
mod common_test_utils {
    use crate::procfs::parsers::{PidStat, Stat};

    pub fn create_stat(running_time: u64) -> Stat {
        // Creates a Stat structure indicating that the CPU has been running for `running_time`
        // ticks
        let individual_ticks = running_time / 6;
        let leftover = running_time - 6 * individual_ticks;

        Stat::new(individual_ticks, individual_ticks, individual_ticks,
                  individual_ticks, individual_ticks,
                  individual_ticks + leftover)
    }

    pub fn create_pid_stat(running_time: u32) -> PidStat {
        // Same operation as above but returns a PidStat instance
        let individual_ticks = (running_time / 4) as u32;
        let leftover = (running_time - 4 * individual_ticks) as u32;

        PidStat::new(individual_ticks, individual_ticks,
                     individual_ticks as i32, (individual_ticks + leftover) as i32)
    }
}