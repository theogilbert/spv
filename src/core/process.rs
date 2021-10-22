//! Process discovery utilities

use log::warn;

use crate::core::Error;

/// Represents the unique ID of a running process
///
/// On Linux 64 bits, the maximum value for a PID is 4194304, hence u32
pub type Pid = u32; // TODO add new type UPID (Unique PID) through the entire execution of spv

/// Basic metadata of a process (PID, command, etc...)
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ProcessMetadata {
    pid: Pid,
    command: String,
}

/// Describes a process
impl ProcessMetadata {
    /// Returns a new instance of a ProcessMetadata
    pub fn new<T>(pid: Pid, command: T) -> ProcessMetadata
    where
        T: Into<String>,
    {
        ProcessMetadata {
            pid,
            command: command.into(),
        }
    }

    /// Returns the process identifier assigned to the process by the OS
    ///
    /// Whilst a PID can be recycled, two running processes can not share the same PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Returns the command used to execute the given process
    ///
    /// This method does not return the arguments passed to the command
    pub fn command(&self) -> &str {
        self.command.as_str()
    }
}

#[cfg(test)]
mod test_process_metadata {
    use crate::core::process::ProcessMetadata;

    #[test]
    fn test_pid_should_be_pm_pid() {
        assert_eq!(ProcessMetadata::new(123, "command").pid(), 123);
    }

    #[test]
    fn test_command_should_be_pm_command() {
        assert_eq!(ProcessMetadata::new(123, "command").command(), "command");
    }
}

/// Collects the running processes
pub struct ProcessCollector {
    scanner: Box<dyn ProcessScanner>,
}

impl ProcessCollector {
    pub fn new(scanner: Box<dyn ProcessScanner>) -> Self {
        Self { scanner }
    }

    /// Returns a Vec of [`ProcessMetadata`](struct.ProcessMetadata), each corresponding to a running process
    pub fn processes(&self) -> Result<Vec<ProcessMetadata>, Error> {
        let pids = self.scanner.scan()?;

        Ok(pids
            .iter()
            .filter_map(|pid| match self.scanner.fetch_metadata(*pid) {
                Err(e) => {
                    warn!("Error fetching process metadata: {:?}", e);
                    None
                }
                Ok(pm) => Some(pm),
            })
            .collect())
    }
}

/// Trait with methods to retrieve basic information about running processes
pub trait ProcessScanner {
    /// Returns a list containing the PIDs of all currently running processes
    fn scan(&self) -> Result<Vec<Pid>, Error>;

    /// Returns The ProcessMetadata of the currently running process with the given PID
    ///
    /// # Arguments
    ///
    /// * `pid`: The process identifier of the currently running process
    fn fetch_metadata(&self, pid: Pid) -> Result<ProcessMetadata, Error>;
}

#[cfg(test)]
mod test_process_collector {
    use crate::core::process::{Pid, ProcessCollector, ProcessMetadata, ProcessScanner};
    use crate::core::Error;
    use crate::core::Error::InvalidPID;

    struct ScannerStub {
        scanned_pids: Vec<Pid>,
        failing_processes: Vec<Pid>,
    }

    impl ProcessScanner for ScannerStub {
        fn scan(&self) -> Result<Vec<Pid>, Error> {
            Ok(self.scanned_pids.clone())
        }

        fn fetch_metadata(&self, pid: Pid) -> Result<ProcessMetadata, Error> {
            if self.failing_processes.contains(&pid) {
                Err(InvalidPID(pid))
            } else {
                Ok(ProcessMetadata::new(pid, "command"))
            }
        }
    }

    fn build_process_collector(scanned_pids: Vec<Pid>) -> ProcessCollector {
        let boxed_scanner = Box::new(ScannerStub {
            scanned_pids,
            failing_processes: vec![],
        });
        ProcessCollector::new(boxed_scanner)
    }

    fn build_failing_process_collector(scanned_pids: Vec<Pid>, failing_processes: Vec<Pid>) -> ProcessCollector {
        let boxed_scanner = Box::new(ScannerStub {
            scanned_pids,
            failing_processes,
        });
        ProcessCollector::new(boxed_scanner)
    }

    #[test]
    fn test_should_collect_no_process_when_no_pid_scanned() {
        let collector = build_process_collector(vec![]);
        assert_eq!(collector.processes().unwrap(), vec![]);
    }

    #[test]
    fn test_should_collect_processes_when_pids_are_scanned() {
        let scanned_pids = vec![1, 2, 3];
        let collector = build_process_collector(scanned_pids.clone());
        let processes = collector.processes().unwrap();

        assert_eq!(processes.len(), 3);

        let mut processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();
        processes_pids.sort();

        assert_eq!(processes_pids, scanned_pids);
    }

    #[test]
    fn test_should_ignore_processes_for_which_scanning_fails() {
        let scanned_pids = vec![1, 2, 3];
        let failing_processes = vec![2];
        let collector = build_failing_process_collector(scanned_pids, failing_processes);
        let processes = collector.processes().unwrap();

        let processes_pids = processes.iter().map(|pm| pm.pid).collect::<Vec<Pid>>();

        assert_eq!(processes_pids.len(), 2);
        assert!(!processes_pids.contains(&2))
    }
}
