use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::probe::{Error, Metric, Probe, ProcessMetric, procfs};
use crate::probe::procfs::{PidStat, Stat};
use crate::process::PID;
use crate::values::PercentValue;

pub struct CpuProbe {
    processes_readers: HashMap<PID, procfs::ProcfsReader<procfs::PidStat>>,
    stat_reader: procfs::ProcfsReader<procfs::Stat>,
    calculator: CpuUsageCalculator,
}


impl CpuProbe {
    pub fn new() -> Result<Self, Error> {
        let mut stat_reader = procfs::ProcfsReader::new("stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))?;

        let stat_data = stat_reader.read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        Ok(CpuProbe {
            processes_readers: HashMap::new(),
            stat_reader,
            calculator: CpuUsageCalculator::new(stat_data),
        })
    }

    fn init_process_reader(pid: PID) -> Result<procfs::ProcfsReader<procfs::PidStat>, Error> {
        procfs::ProcfsReader::new_for_pid(pid, "stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))
    }

    fn get_process_reader(&mut self, pid: PID) -> Result<&mut procfs::ProcfsReader<procfs::PidStat>, Error> {
        Ok(match self.processes_readers.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Self::init_process_reader(pid)?)
        })
    }
}

impl Probe for CpuProbe {
    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        self.calculator.update_stat_data(new_stat);

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<ProcessMetric, Error> {
        let mut proc_reader = self.get_process_reader(pid)?;

        let new_pid_stat = proc_reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        self.calculator.calculate_pid_usage(pid, new_pid_stat)
    }
}

struct CpuUsageCalculator {
    processes_prev_stats: HashMap<PID, procfs::PidStat>,
    prev_global_stat: procfs::Stat,
    global_runtime_diff: f64,
}

impl CpuUsageCalculator {
    pub fn new(init_stat_data: procfs::Stat) -> Self {
        CpuUsageCalculator {
            processes_prev_stats: HashMap::new(),
            prev_global_stat: init_stat_data,
            global_runtime_diff: 0.,
        }
    }

    pub fn update_stat_data(&mut self, stat_data: Stat) -> () {
        self.global_runtime_diff = (stat_data.running_time() - self.prev_global_stat.running_time()) as f64;
        self.prev_global_stat = stat_data;
    }

    pub fn calculate_pid_usage(&mut self, pid: PID, pid_stat_data: PidStat) -> Result<ProcessMetric, Error> {
        let last_iter_runtime = match self.processes_prev_stats.get(&pid) {
            Some(stat_data) => stat_data.running_time(),
            None => 0
        };

        let pid_runtime_diff = pid_stat_data.running_time() - last_iter_runtime;
        self.processes_prev_stats.insert(pid, pid_stat_data);

        let ratio = pid_runtime_diff as f64 / self.global_runtime_diff;
        let percent = (100. * ratio) as f32;

        let value = PercentValue::new(percent)
            .or_else(|_e| {
                Err(Error::ProbingError(format!("Invalid CPU usage value for PID {} : {}",
                                                pid, percent)))
            })?;

        Ok(ProcessMetric { pid, value: Metric::CpuUsage(value) })
    }
}
