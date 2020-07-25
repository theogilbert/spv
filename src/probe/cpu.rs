use crate::probe::{ProcessMetric, Error, Metric, Probe, procfs};
use crate::values::PercentValue;
use crate::process::PID;
use crate::probe::procfs::Stat;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

struct ProcessCpuData {
    reader: procfs::ProcfsReader<procfs::PidStat>,
    prev_stat: procfs::PidStat,
}

struct StatData {
    reader: procfs::ProcfsReader<procfs::Stat>,
    prev_stat: procfs::Stat,
    run_time_diff: u64,
}

pub(crate) struct CpuProbe {
    processes_data: HashMap<PID, ProcessCpuData>,
    stat_data: StatData,
}


// Idea for testing: move computing part out of CpuProbe. CpuProble only handles file parts,
// and defers probe/init_iteration calls to a sub computing class. We would only need to test that
// instance
impl CpuProbe {
    pub fn new() -> Result<Self, Error> {
        let mut stat_file = procfs::ProcfsReader::new("stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))?;

        let stat_data = stat_file
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        Ok(CpuProbe {
            processes_data: HashMap::new(),
            stat_data: StatData {
                reader: stat_file,
                prev_stat: stat_data,
                run_time_diff: 0,
            },
        })
    }

    fn create_process_data(pid: PID) -> Result<ProcessCpuData, Error> {
        let mut stat_file = procfs::ProcfsReader::new_for_pid(pid, "stat")
            .or_else(|e| Err(Error::IOError(e.to_string())))?;

        let pid_stat = stat_file
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        Ok(ProcessCpuData { reader: stat_file, prev_stat: pid_stat })
    }

    /// Returns a mut reference to `ProcessCpuData` associated with `pid`. If it does not exists,
    /// this function will create it first.
    ///
    /// # Arguments
    ///  * `pid`: The ID of the process forr which to retrieve the `ProcessCpuData`
    ///
    fn get_process_data(&mut self, pid: PID) -> Result<&mut ProcessCpuData, Error> {
        Ok(match self.processes_data.entry(pid) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Self::create_process_data(pid)?)
        })
    }
}

impl Probe for CpuProbe {
    fn init_iteration(&mut self) -> Result<(), Error> {
        let new_stat: Stat = self.stat_data.reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        self.stat_data.run_time_diff = new_stat.running_time() - self.stat_data.prev_stat.running_time();
        self.stat_data.prev_stat = new_stat;

        Ok(())
    }

    fn probe(&mut self, pid: PID) -> Result<ProcessMetric, Error> {
        let mut proc_data = self.get_process_data(pid)?;

        let new_pid_stat = proc_data.reader
            .read()
            .or_else(|e| Err(Error::ProbingError(e.to_string())))?;

        let pid_runtime_diff = new_pid_stat.running_time() - proc_data.prev_stat.running_time();
        proc_data.prev_stat = new_pid_stat;

        let ratio = pid_runtime_diff as f64 / self.stat_data.run_time_diff as f64;
        let percent = (100. * ratio) as f32;

        let value = PercentValue::new(percent)
            .or_else(|_e| {
                Err(Error::ProbingError(format!("Invalid percent: {}", percent)))
            })?;

        Ok(ProcessMetric { pid, value: Metric::CpuUsage(value) })
    }
}

struct CpuUsageCalculator {
}

impl CpuUsageCalculator {
    fn update_stat_data(&mut self, stat_data: StatData) -> () {

    }
}
