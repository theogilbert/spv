use std::collections::{HashMap, HashSet};

use crate::probe::{Error, Probe, ProcessMetric, procfs};
use crate::probe::procfs::{PidStat, ProcessDataReader, ReadProcessData, ReadSystemData, Stat, SystemDataReader};
use crate::probe::thread::ProbedFrame;
use crate::process::PID;
use crate::values::PercentValue;

pub struct CpuProbe {
    pid_stat_reader: Box<dyn ReadProcessData<PidStat>>,
    stat_reader: Box<dyn ReadSystemData<Stat>>,
    calculator: UsageCalculator,
}

impl CpuProbe {
    pub fn new() -> Result<Self, Error> {
        let mut stat_reader = SystemDataReader::<Stat>::new()
            .map_err(|e| Error::IOError(e.to_string()))?;

        let stat_data = stat_reader.read()
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        Ok(CpuProbe {
            pid_stat_reader: Box::new(ProcessDataReader::<PidStat>::new()),
            stat_reader: Box::new(stat_reader),
            calculator: UsageCalculator::new(stat_data),
        })
    }
}

impl CpuProbe {
    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_reader
            .read()
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        self.calculator.update_stat_data(new_stat);

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<ProcessMetric<PercentValue>, Error> {
        let new_pid_stat = self.pid_stat_reader
            .read(pid)
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        let pct_value = self.calculator.calculate_pid_usage(pid, new_pid_stat)?;
        Ok(ProcessMetric { pid, value: pct_value })
    }
}

impl Probe for CpuProbe {
    fn probe_frame(&mut self, pids: &HashSet<PID>) -> Result<ProbedFrame, Error> {
        self.init_iteration()?;

        let metrics = pids.iter()
            .filter_map(|p| self.probe(*p).ok())
            .collect();

        Ok(ProbedFrame::PercentsFrame(metrics))
    }
}

#[cfg(test)]
mod test_cpu_probe {
    use std::collections::{HashMap, HashSet};

    use crate::probe::{CpuProbe, Probe, ProcessMetric};
    use crate::probe::cpu::common_test_utils::{create_pid_stat, create_stat};
    use crate::probe::cpu::UsageCalculator;
    use crate::probe::procfs::{PidStat, ProcfsError, ReadProcessData, ReadSystemData, Stat};
    use crate::probe::thread::ProbedFrame;
    use crate::process::PID;
    use crate::values::PercentValue;

    struct MemoryPidStatReader {
        pid_stats_seq: HashMap<PID, Result<PidStat, ProcfsError>>
    }

    impl ReadProcessData<PidStat> for MemoryPidStatReader {
        fn read(&mut self, pid: u32) -> Result<PidStat, ProcfsError> {
            self.pid_stats_seq.remove(&pid)
                .unwrap()
        }
    }

    struct MemoryStatReader {
        stat: Option<Stat>
    }

    impl ReadSystemData<Stat> for MemoryStatReader {
        fn read(&mut self) -> Result<Stat, ProcfsError> {
            Ok(self.stat.take().unwrap())
        }
    }


    macro_rules! hashmap (
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     });

    #[test]
    fn test_should_return_empty_frame_when_no_pid() {
        let stat_reader = MemoryStatReader { stat: Some(create_stat(200)) };
        let pid_stat_reader = MemoryPidStatReader { pid_stats_seq: HashMap::new() };

        let mut probe = CpuProbe {
            pid_stat_reader: Box::new(pid_stat_reader),
            stat_reader: Box::new(stat_reader),
            calculator: UsageCalculator::new(create_stat(0)),
        };

        assert_eq!(probe.probe_frame(&HashSet::new()), Ok(ProbedFrame::PercentsFrame(vec![])));
    }


    #[test]
    fn test_should_return_one_frame_when_one_pid() {
        let stat_reader = MemoryStatReader { stat: Some(create_stat(200)) };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => Ok(create_pid_stat(100)))
        };

        let mut probe = CpuProbe {
            pid_stat_reader: Box::new(pid_stat_reader),
            stat_reader: Box::new(stat_reader),
            calculator: UsageCalculator::new(create_stat(0)),
        };

        assert_eq!(probe.probe_frame(&vec![1].into_iter().collect()),
                   Ok(ProbedFrame::PercentsFrame(vec![
                       ProcessMetric { pid: 1, value: PercentValue::new(50.).unwrap() }
                   ])));
    }


    #[test]
    fn test_should_return_two_frames_when_two_pids() {
        let stat_reader = MemoryStatReader { stat: Some(create_stat(200)) };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(1 => Ok(create_pid_stat(50)), 2 => Ok(create_pid_stat(50)))
        };

        let mut probe = CpuProbe {
            pid_stat_reader: Box::new(pid_stat_reader),
            stat_reader: Box::new(stat_reader),
            calculator: UsageCalculator::new(create_stat(0)),
        };

        assert_eq!(probe.probe_frame(&vec![1, 2].into_iter().collect()),
                   Ok(ProbedFrame::PercentsFrame(vec![
                       ProcessMetric { pid: 1, value: PercentValue::new(25.).unwrap() },
                       ProcessMetric { pid: 2, value: PercentValue::new(25.).unwrap() },
                   ])));
    }


    #[test]
    fn test_should_return_ignore_pid_when_probe_returns_err() {
        let stat_reader = MemoryStatReader { stat: Some(create_stat(200)) };
        let pid_stat_reader = MemoryPidStatReader {
            pid_stats_seq: hashmap!(
            1 => Ok(create_pid_stat(50)),
            2 => Err(ProcfsError::IoError("abc".to_string())))
        };

        let mut probe = CpuProbe {
            pid_stat_reader: Box::new(pid_stat_reader),
            stat_reader: Box::new(stat_reader),
            calculator: UsageCalculator::new(create_stat(0)),
        };

        assert_eq!(probe.probe_frame(&vec![1, 2].into_iter().collect()),
                   Ok(ProbedFrame::PercentsFrame(vec![
                       ProcessMetric { pid: 1, value: PercentValue::new(25.).unwrap() },
                   ])));
    }
}

struct UsageCalculator {
    processes_prev_stats: HashMap<PID, procfs::PidStat>,
    prev_global_stat: procfs::Stat,
    global_runtime_diff: f64,
}

impl UsageCalculator {
    pub fn new(init_stat_data: procfs::Stat) -> Self {
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
                               -> Result<PercentValue, Error> {
        let last_iter_runtime = match self.processes_prev_stats.get(&pid) {
            Some(stat_data) => stat_data.running_time(),
            None => 0
        };

        let pid_runtime_diff = pid_stat_data.running_time() - last_iter_runtime;
        self.processes_prev_stats.insert(pid, pid_stat_data);

        let ratio = pid_runtime_diff as f64 / self.global_runtime_diff;
        let percent = (100. * ratio) as f32;

        PercentValue::new(percent)
            .map_err(|_e| {
                Error::ProbingError(format!("Invalid CPU usage value for PID {} : {}",
                                            pid, percent))
            })
    }
}


#[cfg(test)]
mod test_cpu_calculator {
    use crate::probe::cpu::common_test_utils::create_stat;
    use crate::probe::cpu::UsageCalculator;
    use crate::probe::procfs;
    use crate::values::PercentValue;

    fn create_initialized_calc(elapsed_ticks: u64) -> UsageCalculator {
        let mut calc = UsageCalculator::new(create_stat(100));

        calc.update_stat_data(create_stat(100 + elapsed_ticks));

        calc
    }

    #[test]
    fn test_zero_percent_usage() {
        let mut calc = create_initialized_calc(60);

        let pid_stat = procfs::PidStat::new(0, 0, 0, 0);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat).unwrap(),
                   PercentValue::new(0.).unwrap());
    }

    #[test]
    fn test_hundred_percent_usage() {
        let mut calc = create_initialized_calc(123);

        let pid_stat = procfs::PidStat::new(100, 20, 2, 1);

        assert_eq!(calc.calculate_pid_usage(1, pid_stat).unwrap(),
                   PercentValue::new(100.).unwrap());
    }

    #[test]
    fn test_over_hundred_percent_usage() {
        let mut calc = create_initialized_calc(20);

        let pid_stat = procfs::PidStat::new(10, 10, 10, 10);

        // 40 ticks spent by pid, but only 20 by cpu -> 200% cpu usage
        assert!(calc.calculate_pid_usage(1, pid_stat).is_err());
    }
}

#[cfg(test)]
mod common_test_utils {
    use crate::probe::procfs::{PidStat, Stat};

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