use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::probe::{Error, Probe, ProcessMetric, procfs};
use crate::probe::procfs::{PidStat, Stat};
use crate::process::PID;
use crate::values::PercentValue;

type PidStatReader = procfs::ProcfsReader<procfs::PidStat>;
type StatReader = procfs::ProcfsReader<procfs::Stat>;

pub struct CpuProbe {
    processes_readers: HashMap<PID, PidStatReader>,
    stat_reader: StatReader,
    calculator: UsageCalculator,
}


impl CpuProbe {
    pub fn new() -> Result<Self, Error> {
        let mut stat_reader = StatReader::new("stat")
            .map_err(|e| Error::IOError(e.to_string()))?;

        let stat_data = stat_reader.read()
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        Ok(CpuProbe {
            processes_readers: HashMap::new(),
            stat_reader,
            calculator: UsageCalculator::new(stat_data),
        })
    }

    fn init_process_reader(pid: PID) -> Result<PidStatReader, Error> {
        PidStatReader::new_for_pid(pid, "stat")
            .map_err(|e| Error::IOError(e.to_string()))
    }

    fn get_process_reader(&mut self, pid: PID) -> Result<&mut PidStatReader, Error> {
        Ok(match self.processes_readers.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert({
                Self::init_process_reader(pid)?
            })
        })
    }
}

impl Probe for CpuProbe {
    type ValueType = PercentValue;

    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_reader
            .read()
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        self.calculator.update_stat_data(new_stat);

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<ProcessMetric<PercentValue>, Error> {
        let proc_reader = self.get_process_reader(pid)?;

        let new_pid_stat = proc_reader
            .read()
            .map_err(|e| Error::ProbingError(e.to_string()))?;

        let pct_value = self.calculator.calculate_pid_usage(pid, new_pid_stat)?;
        Ok(ProcessMetric { pid, value: pct_value })
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
    use crate::probe::cpu::UsageCalculator;
    use crate::probe::procfs;
    use crate::values::PercentValue;

    fn create_initialized_calc(elapsed_ticks: u64) -> UsageCalculator {
        let first_stat = procfs::Stat::new(1, 2, 3, 4, 5, 6);

        let individual_ticks = elapsed_ticks / 6;
        let leftover = elapsed_ticks - 6 * individual_ticks;

        let second_stat = procfs::Stat::new(1 + individual_ticks,
                                            2 + individual_ticks,
                                            3 + individual_ticks,
                                            4 + individual_ticks,
                                            5 + individual_ticks,
                                            6 + individual_ticks + leftover);

        let mut calc = UsageCalculator::new(first_stat);

        calc.update_stat_data(second_stat);

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